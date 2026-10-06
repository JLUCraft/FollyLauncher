"""Keep Minecraft formats and implementation in minecraftd; the host adapts UI/HTTP."""
from pathlib import Path
import re
import sys

ROOT = Path(__file__).resolve().parents[2]
SOURCE = ROOT / "src-tauri" / "src"

# These dependencies are owned by minecraftd after extracting the associated formats.
FORBIDDEN_DEPENDENCIES = ("cafebabe", "quartz_nbt", "murmur2")
# A re-export is allowed; a second schema or parser is not.
FORBIDDEN_DEFINITIONS = (
    "McClientInfo", "LibrariesValue", "InstructionRule", "LaunchArguments",
    "ModrinthManifest", "MultiMcManifest", "CurseForgeManifest", "NbtServerInfo",
)
# The per-loader mod parsers live in minecraftd; only the thin adapter
# (common.rs/mod.rs) may remain in this directory.
MODS_HELPERS = SOURCE / "instance" / "helpers" / "mods"
MODS_ALLOWED = ("common.rs", "mod.rs")
errors = []
manifest = (ROOT / "src-tauri" / "Cargo.toml").read_text()
for dependency in FORBIDDEN_DEPENDENCIES:
    if re.search(rf"^{dependency}(?:\.|\s*=)", manifest, re.MULTILINE):
        errors.append(f"Cargo.toml: {dependency} belongs to minecraftd")
for path in MODS_HELPERS.glob("*.rs"):
    if path.name not in MODS_ALLOWED:
        errors.append(f"{path.relative_to(SOURCE)}: extracted parser must not return")
definitions = re.compile(r"\b(?:struct|enum)\s+(" + "|".join(FORBIDDEN_DEFINITIONS) + r")\b")
for path in SOURCE.rglob("*.rs"):
    match = definitions.search(path.read_text())
    if match:
        errors.append(f"{path.relative_to(SOURCE)}: duplicate {match.group(1)}")
for relative in ("resource/helpers/loader_meta", "instance/helpers/loader"):
    for path in (SOURCE / relative).rglob("*.rs"):
        if "https://" in path.read_text():
            errors.append(f"{path.relative_to(SOURCE)}: loader endpoint belongs to minecraftd")
if errors:
    print("\n".join(errors), file=sys.stderr)
    sys.exit(1)
print("Minecraft boundary checks passed")
