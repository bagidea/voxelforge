# World node — owns all chunk MeshInstance3Ds, generates terrain around the
# player, and provides the public block-edit API (place / break).

extends Node3D

const _BlockTypes   = preload("res://scripts/block_types.gd")
const _Chunk        = preload("res://scripts/chunk.gd")
const _Mesher       = preload("res://scripts/chunk_mesher.gd")
const _TextureAtlas = preload("res://scripts/texture_atlas.gd")

const VIEW_RADIUS := 3

var _chunks      : Dictionary = {}   # Vector3i → MeshInstance3D
var _chunk_data  : Dictionary = {}   # Vector3i → RefCounted (ChunkData)
var _material    : StandardMaterial3D


func _ready() -> void:
	var atlas_tex := _TextureAtlas.build()
	_material = StandardMaterial3D.new()
	_material.albedo_texture = atlas_tex
	_material.texture_filter = BaseMaterial3D.TEXTURE_FILTER_NEAREST
	_material.shading_mode = BaseMaterial3D.SHADING_MODE_UNSHADED


func update_around(player_world_pos: Vector3) -> void:
	var cs := _Chunk.CHUNK_SIZE
	var pcx := floori(player_world_pos.x / cs)
	var pcz := floori(player_world_pos.z / cs)

	var wanted := {}
	for dx in range(-VIEW_RADIUS, VIEW_RADIUS + 1):
		for dz in range(-VIEW_RADIUS, VIEW_RADIUS + 1):
			var cpos := Vector3i(pcx + dx, 0, pcz + dz)
			wanted[cpos] = true

	var to_remove := []
	for cpos in _chunks:
		if not wanted.has(cpos):
			to_remove.append(cpos)
	for cpos in to_remove:
		_remove_chunk(cpos)

	for cpos in wanted:
		if not _chunks.has(cpos):
			_add_chunk(cpos)


func place_block(block_id: int, hit_pos: Vector3, hit_normal: Vector3) -> bool:
	var place_voxel := Vector3i(
		floori(hit_pos.x + hit_normal.x * 0.5),
		floori(hit_pos.y + hit_normal.y * 0.5),
		floori(hit_pos.z + hit_normal.z * 0.5),
	)
	return _set_block(place_voxel, block_id)


func break_block(hit_pos: Vector3, hit_normal: Vector3) -> bool:
	var voxel := Vector3i(
		floori(hit_pos.x - hit_normal.x * 0.5),
		floori(hit_pos.y - hit_normal.y * 0.5),
		floori(hit_pos.z - hit_normal.z * 0.5),
	)
	return _set_block(voxel, _BlockTypes.AIR)


func get_block_at(world_pos: Vector3i) -> int:
	var cs := _Chunk.CHUNK_SIZE
	var cx := floori(float(world_pos.x) / cs)
	var cy := floori(float(world_pos.y) / cs)
	var cz := floori(float(world_pos.z) / cs)
	var cpos := Vector3i(cx, cy, cz)

	var chunk_data = _chunk_data.get(cpos)
	if chunk_data == null:
		return _BlockTypes.AIR

	var lx := world_pos.x - cx * cs
	var ly := world_pos.y - cy * cs
	var lz := world_pos.z - cz * cs
	return chunk_data.get_block(lx, ly, lz)


func chunk_count() -> int:
	return _chunks.size()


# ---- internals ----------------------------------------------------------

func _set_block(world_pos: Vector3i, block_id: int) -> bool:
	var cs := _Chunk.CHUNK_SIZE
	var cx := floori(float(world_pos.x) / cs)
	var cy := floori(float(world_pos.y) / cs)
	var cz := floori(float(world_pos.z) / cs)
	var cpos := Vector3i(cx, cy, cz)

	var chunk_data = _chunk_data.get(cpos)
	if chunk_data == null:
		return false

	var lx := world_pos.x - cx * cs
	var ly := world_pos.y - cy * cs
	var lz := world_pos.z - cz * cs
	chunk_data.set_block(lx, ly, lz, block_id)

	_remesh_chunk(cpos)

	if lx == 0:
		_remesh_chunk(Vector3i(cx - 1, cy, cz))
	elif lx == cs - 1:
		_remesh_chunk(Vector3i(cx + 1, cy, cz))
	if ly == 0:
		_remesh_chunk(Vector3i(cx, cy - 1, cz))
	elif ly == cs - 1:
		_remesh_chunk(Vector3i(cx, cy + 1, cz))
	if lz == 0:
		_remesh_chunk(Vector3i(cx, cy, cz - 1))
	elif lz == cs - 1:
		_remesh_chunk(Vector3i(cx, cy, cz + 1))

	return true


func _add_chunk(cpos: Vector3i) -> void:
	var cd := _Chunk.generate(cpos)
	_chunk_data[cpos] = cd

	var result: Dictionary = _Mesher.greedy_mesh(cd)
	var mesh: ArrayMesh = result["mesh"]

	var mi := MeshInstance3D.new()
	mi.name = "Chunk_%d_%d_%d" % [cpos.x, cpos.y, cpos.z]
	mi.mesh = mesh
	mi.material_override = _material
	mi.position = Vector3(cpos.x * _Chunk.CHUNK_SIZE, cpos.y * _Chunk.CHUNK_SIZE, cpos.z * _Chunk.CHUNK_SIZE)
	_build_collision(mi, mesh)
	add_child(mi)
	_chunks[cpos] = mi


func _remove_chunk(cpos: Vector3i) -> void:
	var mi: MeshInstance3D = _chunks.get(cpos)
	if mi != null:
		mi.queue_free()
	_chunks.erase(cpos)
	_chunk_data.erase(cpos)


func _remesh_chunk(cpos: Vector3i) -> void:
	var cd = _chunk_data.get(cpos)
	if cd == null:
		return

	var result: Dictionary = _Mesher.greedy_mesh(cd)
	var mesh: ArrayMesh = result["mesh"]

	var mi: MeshInstance3D = _chunks.get(cpos)
	if mi == null:
		mi = MeshInstance3D.new()
		mi.name = "Chunk_%d_%d_%d" % [cpos.x, cpos.y, cpos.z]
		mi.material_override = _material
		mi.position = Vector3(cpos.x * _Chunk.CHUNK_SIZE, cpos.y * _Chunk.CHUNK_SIZE, cpos.z * _Chunk.CHUNK_SIZE)
		add_child(mi)
		_chunks[cpos] = mi

	mi.mesh = mesh
	_build_collision(mi, mesh)


## Build / replace a StaticBody3D + ConcavePolygonShape3D child on `mi`.
func _build_collision(mi: MeshInstance3D, mesh: ArrayMesh) -> void:
	# Remove old collision node if present.
	for child in mi.get_children():
		if child is StaticBody3D:
			child.queue_free()

	# Nothing to collide with — all air.
	if mesh == null:
		return

	# Gather all triangle faces from the greedy mesh.
	var faces := PackedVector3Array()
	for s: int in mesh.get_surface_count():
		var arrs: Array = mesh.surface_get_arrays(s)
		var verts: PackedVector3Array = arrs[Mesh.ARRAY_VERTEX]
		var idxs: PackedInt32Array = arrs[Mesh.ARRAY_INDEX]
		if verts == null or verts.size() == 0:
			continue

		if idxs != null and idxs.size() > 0:
			for i: int in range(0, idxs.size(), 3):
				faces.append(verts[idxs[i]])
				faces.append(verts[idxs[i + 1]])
				faces.append(verts[idxs[i + 2]])
		else:
			# No index buffer — triangles are sequential.
			for i: int in range(0, verts.size(), 3):
				faces.append(verts[i])
				faces.append(verts[i + 1])
				faces.append(verts[i + 2])

	if faces.size() == 0:
		return

	var shape := ConcavePolygonShape3D.new()
	shape.set_faces(faces)

	var cs := CollisionShape3D.new()
	cs.shape = shape

	var sb := StaticBody3D.new()
	sb.name = "Collision"
	sb.add_child(cs)
	mi.add_child(sb)
