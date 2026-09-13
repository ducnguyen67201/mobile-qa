//! Deterministic checks read retained evidence. SDK prose and exit status are not assertions.
use super::{execution_store::*, run_artifacts};
use crate::errors::ApiResult;
use loco_rs::app::AppContext;
use mobile_qa_contracts::execution::*;
use quick_xml::{events::Event, Reader};
use std::collections::BTreeMap;
use uuid::Uuid;

pub fn observe(
    xml: &[u8],
    package: &str,
    check: &ExpectedCheck,
    task: &str,
) -> Result<String, &'static str> {
    if xml.is_empty() || xml.len() > 2097152 {
        return Err("invalid_hierarchy");
    }
    let mut reader = Reader::from_reader(xml);
    let mut depth = 0u32;
    let mut count = 0u32;
    let mut nodes = vec![];
    let mut root = false;
    loop {
        let event = reader.read_event().map_err(|_| "invalid_hierarchy")?;
        let start = matches!(&event, Event::Start(_));
        match event {
            Event::DocType(_) => return Err("invalid_hierarchy"),
            Event::Start(e) | Event::Empty(e) => {
                let name = e.name();
                if root && depth == 0 {
                    return Err("invalid_hierarchy");
                }
                if !root {
                    if name.as_ref() != b"hierarchy" {
                        return Err("invalid_hierarchy");
                    }
                    root = true;
                }
                if name.as_ref() == b"node" {
                    count += 1;
                    if count > 10000 {
                        return Err("invalid_hierarchy");
                    }
                    let mut values = BTreeMap::new();
                    for attr in e.attributes() {
                        let attr = attr.map_err(|_| "invalid_hierarchy")?;
                        let key = std::str::from_utf8(attr.key.as_ref())
                            .map_err(|_| "invalid_hierarchy")?
                            .to_owned();
                        let value = attr
                            .decoded_and_normalized_value(
                                quick_xml::XmlVersion::Implicit1_0,
                                reader.decoder(),
                            )
                            .map_err(|_| "invalid_hierarchy")?
                            .into_owned();
                        values.insert(key, value);
                    }
                    if values.get("package").map(String::as_str) == Some(package) {
                        nodes.push(values);
                    }
                }
                if start {
                    depth += 1;
                    if depth > 128 {
                        return Err("invalid_hierarchy");
                    }
                }
            }
            Event::End(_) => depth = depth.saturating_sub(1),
            Event::Eof => break,
            _ => {}
        }
    }
    if depth != 0 {
        return Err("invalid_hierarchy");
    }
    if !root || nodes.is_empty() {
        return Err("wrong_foreground_package");
    }
    if nodes.iter().any(|n| {
        n.get("resource-id").map(String::as_str)
            == Some(&format!("{package}:id/prerequisite_unavailable"))
    }) {
        return Err("prerequisite_unavailable");
    }
    if !nodes
        .iter()
        .any(|n| n.get("resource-id") == Some(&check.ready_resource_id))
    {
        return Err("screen_not_ready");
    }
    let mut targets: Vec<_> = nodes
        .iter()
        .filter(|n| n.get("resource-id") == Some(&check.resource_id))
        .collect();
    if !check.text_filter.is_empty() {
        let text = check.text_filter.replace("${task_title}", task);
        targets.retain(|n| n.get("text") == Some(&text));
    }
    // Ambiguous evidence must not become unique solely because one value matches the expectation.

    if check.method == CheckMethod::UiElementPresenceV1 {
        return Ok((!targets.is_empty()).to_string());
    }
    if targets.len() != 1 {
        return Err("target_not_unique");
    }
    let property = match check.property {
        UiProperty::Text => "text",
        UiProperty::ContentDescription => "content-desc",
        UiProperty::Checked => "checked",
        UiProperty::Enabled => "enabled",
    };
    targets[0].get(property).cloned().ok_or("property_missing")
}

pub fn png_valid(data: &[u8]) -> bool {
    if data.len() < 45 || data.len() > 16777216 || !data.starts_with(b"\x89PNG\r\n\x1a\n") {
        return false;
    }
    let mut offset = 8;
    let mut image = false;
    let mut header = false;
    while offset + 12 <= data.len() {
        let size =
            u32::from_be_bytes(data[offset..offset + 4].try_into().expect("four bytes")) as usize;
        let Some(end) = offset.checked_add(size + 12) else {
            return false;
        };
        if end > data.len() {
            return false;
        }
        let kind = &data[offset + 4..offset + 8];
        let mut crc = 0xffffffffu32;
        for b in &data[offset + 4..end - 4] {
            crc ^= u32::from(*b);
            for _ in 0..8 {
                crc = (crc >> 1) ^ if crc & 1 == 1 { 0xedb88320 } else { 0 };
            }
        }
        if !crc != u32::from_be_bytes(data[end - 4..end].try_into().expect("four bytes")) {
            return false;
        }
        if kind == b"IHDR" {
            if offset != 8
                || size != 13
                || data[offset + 8..offset + 12] != 1080u32.to_be_bytes()
                || data[offset + 12..offset + 16] != 1920u32.to_be_bytes()
            {
                return false;
            }
            header = true;
        }
        if kind == b"IDAT" {
            image = true;
        }
        if kind == b"IEND" {
            return size == 0 && end == data.len() && header && image;
        }
        offset = end;
    }
    false
}

pub async fn evaluate(
    ctx: &AppContext,
    id: Uuid,
    case: &CaseDefinition,
) -> ApiResult<(Vec<CheckResult>, Outcome)> {
    let artifacts = rows(
        &ctx.db,
        "SELECT * FROM execution_artifacts WHERE attempt_id=$1 AND state='sealed'",
        vec![id.into()],
    )
    .await?
    .iter()
    .map(run_artifacts::record)
    .collect::<ApiResult<Vec<_>>>()?;
    let task = format!("qa-{id}");
    let mut results: Vec<CheckResult> = vec![];
    for check in &case.checks {
        let expected = check.expected.replace("${task_title}", &task);
        let mut result = CheckResult {
            check_id: check.id.clone(),
            expected: expected.clone(),
            observed: None,
            outcome: Outcome::Inconclusive,
            reason: "required_evidence_missing".into(),
            artifact_ids: vec![],
        };
        if check.prerequisite_check_ids.iter().any(|p| {
            !results
                .iter()
                .any(|r| &r.check_id == p && r.outcome == Outcome::Passed)
        }) {
            result.reason = "prerequisite_not_proven".into();
            results.push(result);
            continue;
        }
        let xml = artifacts.iter().find(|a| {
            a.checkpoint_id == check.checkpoint_id
                && a.name == format!("{}.xml", check.checkpoint_id)
        });
        let png = artifacts.iter().find(|a| {
            a.checkpoint_id == check.checkpoint_id
                && a.name == format!("{}.png", check.checkpoint_id)
        });
        if let (Some(xml), Some(png)) = (xml, png) {
            result.artifact_ids = vec![xml.id, png.id];
            if let (Ok((_, x)), Ok((_, p))) = (
                run_artifacts::content(ctx, xml.id).await,
                run_artifacts::content(ctx, png.id).await,
            ) {
                let xml_bytes = tokio::fs::read(&x.0).await?;
                let png_bytes = tokio::fs::read(&p.0).await?;
                if png_valid(&png_bytes) && check.method != CheckMethod::Manual {
                    match observe(&xml_bytes, &case.package, check, &task) {
                        Ok(actual) => {
                            result.outcome = if actual == expected {
                                Outcome::Passed
                            } else {
                                Outcome::Failed
                            };
                            result.reason = if actual == expected {
                                "expected_value_observed"
                            } else {
                                "expected_value_contradicted"
                            }
                            .into();
                            result.observed = Some(actual);
                        }
                        Err(reason) => {
                            result.reason = reason.into();
                            if reason == "prerequisite_unavailable" {
                                result.outcome = Outcome::Blocked;
                            }
                        }
                    }
                } else {
                    result.reason = "invalid_or_unsupported_evidence".into();
                }
            }
        }
        results.push(result);
    }
    let required: Vec<_> = case
        .checks
        .iter()
        .zip(&results)
        .filter(|(c, _)| c.required)
        .map(|(_, r)| r.outcome)
        .collect();
    let outcome = if required.contains(&Outcome::Failed) {
        Outcome::Failed
    } else if required.contains(&Outcome::Blocked) {
        Outcome::Blocked
    } else if !required.is_empty() && required.iter().all(|o| *o == Outcome::Passed) {
        Outcome::Passed
    } else {
        Outcome::Inconclusive
    };
    Ok((results, outcome))
}
