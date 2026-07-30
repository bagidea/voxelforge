# Greedy voxel mesher — builds an ArrayMesh from ChunkData.
# Use: const Mesher = preload("res://scripts/chunk_mesher.gd"); Mesher.greedy_mesh(chunk)

const CS := 32
const AIR := 0
const TILE_COUNT : int = 4
const TILE_SIZE  : float = 16.0
const INSET      : float = 0.5 / (TILE_COUNT * TILE_SIZE)


## chunk must be an instance from preload("res://scripts/chunk.gd")
static func greedy_mesh(chunk: RefCounted) -> Dictionary:
	var dims := [CS, CS, CS]
	var positions := PackedVector3Array()
	var normals   := PackedVector3Array()
	var uvs       := PackedVector2Array()
	var indices   := PackedInt32Array()
	var quads     := 0

	for d in 3:
		var u := (d + 1) % 3
		var v := (d + 2) % 3
		var x  := [0, 0, 0]
		var q  := [0, 0, 0]
		q[d] = 1

		var mask := PackedInt32Array()
		mask.resize(dims[u] * dims[v])

		x[d] = -1
		while x[d] < dims[d]:
			var n := 0
			for j in dims[v]:
				for i in dims[u]:
					x[u] = i
					x[v] = j
					var a: int = chunk.get_block(x[0], x[1], x[2])
					var b: int = chunk.get_block(x[0] + q[0], x[1] + q[1], x[2] + q[2])
					var sa: bool = a != AIR
					var sb: bool = b != AIR
					if sa == sb:
						mask[n] = 0
					elif sa:
						mask[n] = a
					else:
						mask[n] = -b
					n += 1

			x[d] += 1

			n = 0
			var j := 0
			while j < dims[v]:
				var i := 0
				while i < dims[u]:
					var c := mask[n]
					if c != 0:
						var w := 1
						while i + w < dims[u] and mask[n + w] == c:
							w += 1
						var hh := 1
						var growing := true
						while growing and j + hh < dims[v]:
							for k in w:
								if mask[n + k + hh * dims[u]] != c:
									growing = false
									break
							if growing:
								hh += 1

						x[u] = i
						x[v] = j
						var du := Vector3.ZERO
						du[u] = float(w)
						var dv := Vector3.ZERO
						dv[v] = float(hh)
						var p0 := Vector3(float(x[0]), float(x[1]), float(x[2]))

						var front : bool = c > 0
						var block_id : int = absi(c)
						var nrm := Vector3.ZERO
						nrm[d] = 1.0 if front else -1.0

						var base_i := positions.size()
						positions.append(p0)
						positions.append(p0 + du)
						positions.append(p0 + du + dv)
						positions.append(p0 + dv)

						normals.append(nrm)
						normals.append(nrm)
						normals.append(nrm)
						normals.append(nrm)

						var uv: Array = _block_uvs(block_id)
						if front:
							uvs.append(uv[0]); uvs.append(uv[1])
							uvs.append(uv[2]); uvs.append(uv[3])
							indices.append_array([
								base_i, base_i + 1, base_i + 2,
								base_i, base_i + 2, base_i + 3,
							])
						else:
							uvs.append(uv[0]); uvs.append(uv[1])
							uvs.append(uv[2]); uvs.append(uv[3])
							indices.append_array([
								base_i, base_i + 2, base_i + 1,
								base_i, base_i + 3, base_i + 2,
							])

						quads += 1

						for l in hh:
							for k in w:
								mask[n + k + l * dims[u]] = 0
						i += w
						n += w
					else:
						i += 1
						n += 1
				j += 1

	var arrays := []
	arrays.resize(Mesh.ARRAY_MAX)
	arrays[Mesh.ARRAY_VERTEX] = positions
	arrays[Mesh.ARRAY_NORMAL]  = normals
	arrays[Mesh.ARRAY_TEX_UV]  = uvs
	arrays[Mesh.ARRAY_INDEX]   = indices

	var mesh := ArrayMesh.new()
	mesh.add_surface_from_arrays(Mesh.PRIMITIVE_TRIANGLES, arrays)
	return { "mesh": mesh, "quads": quads }


static func _block_uvs(block_id: int) -> Array:
	var t: float = float(block_id - 1)
	var u0: float = t / TILE_COUNT + INSET
	var u1: float = (t + 1.0) / TILE_COUNT - INSET
	return [
		Vector2(u0, 0.0),
		Vector2(u1, 0.0),
		Vector2(u1, 1.0),
		Vector2(u0, 1.0),
	]
