# Block type constants — mirrors sim/src/block.rs palette.
# AIR = 0, opaque blocks = 1..255

const AIR   : int = 0
const GRASS : int = 1
const DIRT  : int = 2
const STONE : int = 3
const SAND  : int = 4

# Human-readable names
const NAMES : Dictionary = {
	AIR:   "Air",
	GRASS: "Grass",
	DIRT:  "Dirt",
	STONE: "Stone",
	SAND:  "Sand",
}

# Base colours (sRGB) per block type — used for the procedural atlas.
const COLORS : Dictionary = {
	GRASS: Color(0.275, 0.627, 0.259),
	DIRT:  Color(0.486, 0.345, 0.220),
	STONE: Color(0.502, 0.502, 0.541),
	SAND:  Color(0.839, 0.792, 0.580),
}

const ALL_PLACEABLE : Array[int] = [GRASS, DIRT, STONE, SAND]

static func is_opaque(id: int) -> bool:
	return id != AIR

static func name_of(id: int) -> String:
	return NAMES.get(id, "Unknown")
