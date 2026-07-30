# Dense 32³ voxel storage for one chunk.
#
# Layout: blocks[x + CHUNK_SIZE * (z + CHUNK_SIZE * y)] — Y-major.
# Use: const Chunk = preload("res://scripts/chunk.gd"); Chunk.generate(pos)

extends RefCounted

const _BlockTypes   = preload("res://scripts/block_types.gd")
const _TerrainNoise = preload("res://scripts/terrain_noise.gd")
const _Self         = preload("res://scripts/chunk.gd")

const CHUNK_SIZE : int = 32
const VOLUME     : int = CHUNK_SIZE * CHUNK_SIZE * CHUNK_SIZE

var pos     : Vector3i
var blocks  : PackedByteArray


static func generate(p: Vector3i) -> RefCounted:
	var cd := _Self.new()
	cd.pos = p
	cd.blocks = PackedByteArray()
	cd.blocks.resize(VOLUME)

	var ox := p.x * CHUNK_SIZE
	var oy := p.y * CHUNK_SIZE
	var oz := p.z * CHUNK_SIZE

	if p.y < 0:
		cd.blocks.fill(_BlockTypes.STONE)
	elif p.y == 0:
		for z in CHUNK_SIZE:
			for x in CHUNK_SIZE:
				var wx := float(ox + x)
				var wz := float(oz + z)
				var h := _TerrainNoise.height(wx, wz)

				var max_y := h + 1
				if max_y > CHUNK_SIZE:
					max_y = CHUNK_SIZE
				for y in range(0, max_y):
					var world_y := oy + y
					var b: int
					if y == h:
						if world_y < 11:
							b = _BlockTypes.SAND
						else:
							b = _BlockTypes.GRASS
					elif y > h - 4:
						b = _BlockTypes.DIRT
					else:
						b = _BlockTypes.STONE
					cd.blocks[cd._idx(x, y, z)] = b
	return cd


static func empty(p: Vector3i) -> RefCounted:
	var cd := _Self.new()
	cd.pos = p
	cd.blocks = PackedByteArray()
	cd.blocks.resize(VOLUME)
	return cd


func get_block(x: int, y: int, z: int) -> int:
	if x < 0 or y < 0 or z < 0 or x >= CHUNK_SIZE or y >= CHUNK_SIZE or z >= CHUNK_SIZE:
		return _BlockTypes.AIR
	return blocks[_idx(x, y, z)]


func set_block(x: int, y: int, z: int, block: int) -> void:
	if x < 0 or y < 0 or z < 0 or x >= CHUNK_SIZE or y >= CHUNK_SIZE or z >= CHUNK_SIZE:
		return
	blocks[_idx(x, y, z)] = block


func solid_count() -> int:
	var n := 0
	for b in blocks:
		if _BlockTypes.is_opaque(b):
			n += 1
	return n


func _idx(x: int, y: int, z: int) -> int:
	return x + CHUNK_SIZE * (z + CHUNK_SIZE * y)
