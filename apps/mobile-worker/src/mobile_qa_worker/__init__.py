"""Mobile worker package: deterministic fixtures today, real device work later.

Importing this package starts no worker, imports no Minitap SDK, and contacts no phone.
Device qualification is spec 02; durable job integration is spec 04. The fake path
stays useful for fast tests after a production adapter exists.
"""
