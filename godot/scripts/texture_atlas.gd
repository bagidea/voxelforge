# Procedural 4-tile texture atlas — mirrors client/src/voxel.rs build_atlas().
# Use: const Atlas = preload("res://scripts/texture_atlas.gd"); Atlas.build()

const TILE_COUNT : int = 4
const TILE_SIZE  : int = 16

static func build() -> ImageTexture:
	var w: int = TILE_COUNT * TILE_SIZE
	var h: int = TILE_SIZE
	var img: Image = Image.create(w, h, false, Image.FORMAT_RGBA8)

	var bases: Array[Color] = [
		Color(0.275, 0.627, 0.259),   # GRASS
		Color(0.486, 0.345, 0.220),   # DIRT
		Color(0.502, 0.502, 0.541),   # STONE
		Color(0.839, 0.792, 0.580),   # SAND
	]

	for ty in h:
		for tx in w:
			var tile: int = tx / TILE_SIZE
			var lx: int = tx % TILE_SIZE
			var base: Color = bases[tile]

			var hsh: int = (tx * 374761393) ^ (ty * 668265263)
			hsh = hsh * 1274126177
			var noise: int = (hsh >> 24) & 0xff
			var n: int = noise % 26 - 13

			var border: bool = lx == 0 or ty == 0 or lx == TILE_SIZE - 1 or ty == TILE_SIZE - 1
			var shade: int = -40 if border else n

			var r: int = clampi(int(base.r * 255.0) + shade, 0, 255)
			var g: int = clampi(int(base.g * 255.0) + shade, 0, 255)
			var b: int = clampi(int(base.b * 255.0) + shade, 0, 255)

			img.set_pixel(tx, ty, Color(r / 255.0, g / 255.0, b / 255.0, 1.0))

	var tex: ImageTexture = ImageTexture.create_from_image(img)
	tex.resource_name = "voxel_atlas"
	return tex
