"""Sanitize the retained screen and hierarchy before reporting or model access."""

import io
import re
from xml.etree import ElementTree

from mobile_qa_worker.qualification.config import QualificationError


def sanitize(xml: bytes, png: bytes, package: str | None = None) -> tuple[bytes, bytes]:
    from PIL import Image, ImageDraw

    if not xml or len(xml) > 1048576 or b"<!" in xml:
        raise QualificationError("redaction_failed")
    try:
        tree = ElementTree.fromstring(xml)
        image = Image.open(io.BytesIO(png))
        if tree.tag != "hierarchy" or image.size != (1080, 1920):
            raise QualificationError("redaction_failed")
        draw = ImageDraw.Draw(image)
        for node in tree.iter("node"):
            sensitive = node.get("password") == "true"
            foreign = package is not None and node.get("package") != package
            if sensitive:
                bounds = re.fullmatch(r"\[(\d+),(\d+)\]\[(\d+),(\d+)\]", node.get("bounds", ""))
                if not bounds:
                    raise QualificationError("redaction_failed")
                left, top, right, bottom = map(int, bounds.groups())
                if not (0 <= left < right <= 1080 and 0 <= top < bottom <= 1920):
                    raise QualificationError("redaction_failed")
                draw.rectangle((left, top, right, bottom), fill="black")
            if sensitive or foreign:
                for key in ("text", "content-desc", "resource-id"):
                    node.set(key, "")
        output = io.BytesIO()
        image.save(output, format="PNG")
        return ElementTree.tostring(tree), output.getvalue()
    except Exception as exc:
        raise QualificationError("redaction_failed") from exc
