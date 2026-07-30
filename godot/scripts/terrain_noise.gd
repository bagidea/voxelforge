# Deterministic FBM terrain height — ported from sim/src/worldgen.rs.
# Hash constants adjusted for GDScript's signed 64-bit int range.

const CHUNK_SIZE := 32
const OCTAVES     : int = 5
const PERSISTENCE : float = 0.55
const LACUNARITY  : float = 2.3
const BASE_FREQ   : float = 0.007
const HEIGHT_MIN  : float = 3.0
const HEIGHT_MAX  : float = 29.0

# Equivalent bit-patterns as signed 64-bit ints
const HASH_MUL1 : int = 6364136223846793005   # 0x5851f42d4c957f2d (fits in i64)
const HASH_MUL2 : int = -45569865847948083    # 0xff51afd7ed558ccd as signed
const HASH_MASK : int = -1                    # 0xffffffffffffffff
const GOLDEN    : int = -7046029254386353131  # 0x9e3779b97f4a7c15 as signed

static var _seed : int = 42


static func set_seed(s: int) -> void:
	_seed = s


static func _hash(x: int, z: int, s: int) -> int:
	var h : int = s
	h = (h * HASH_MUL1) + x
	h = ((h ^ (h >> 33)) * HASH_MUL2)
	h = (h * HASH_MUL1) + z
	h = ((h ^ (h >> 33)) * HASH_MUL2)
	h = h ^ (h >> 33)
	return ((h >> 32) ^ h) & 0xffffffff


static func _value_2d(x: int, z: int, s: int) -> float:
	return float(_hash(x, z, s)) / 4294967296.0


static func _smooth_noise(wx: float, wz: float, s: int) -> float:
	var x0 := floori(wx)
	var z0 := floori(wz)
	var fx: float = wx - float(x0)
	var fz: float = wz - float(z0)
	var sx: float = fx * fx * (3.0 - 2.0 * fx)
	var sz: float = fz * fz * (3.0 - 2.0 * fz)

	var v00: float = _value_2d(x0, z0, s)
	var v10: float = _value_2d(x0 + 1, z0, s)
	var v01: float = _value_2d(x0, z0 + 1, s)
	var v11: float = _value_2d(x0 + 1, z0 + 1, s)

	var v0: float = v00 + sx * (v10 - v00)
	var v1: float = v01 + sx * (v11 - v01)
	return v0 + sz * (v1 - v0)


static func _fbm(wx: float, wz: float, base_seed: int) -> float:
	var total := 0.0
	var freq := BASE_FREQ
	var amp := 1.0
	var max_amp := 0.0
	var octave_seed := base_seed

	for _i in OCTAVES:
		total += _smooth_noise(wx * freq, wz * freq, octave_seed) * amp
		max_amp += amp
		freq *= LACUNARITY
		amp  *= PERSISTENCE
		octave_seed += GOLDEN

	var result: float = total / max_amp if max_amp > 0.0 else total
	return result


static func height(wx: float, wz: float) -> int:
	var n: float = _fbm(wx, wz, _seed)
	var h: float = HEIGHT_MIN + n * (HEIGHT_MAX - HEIGHT_MIN)
	return clampi(roundi(h), 1, CHUNK_SIZE - 2)
