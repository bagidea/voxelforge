# FPS-style player controller with block place/break via raycast.
# Temporary controller for testing the voxel world.

extends CharacterBody3D

const _BlockTypes = preload("res://scripts/block_types.gd")

@export var mouse_sensitivity : float = 0.002
@export var move_speed        : float = 12.0
@export var jump_velocity     : float = 8.0
@export var reach_distance    : float = 8.0
@export var selected_block    : int   = 1  # GRASS

@onready var _camera    : Camera3D       = $Camera3D
@onready var _world                      = $"../World"
@onready var _highlight : MeshInstance3D = $Camera3D/HighlightCube
@onready var _ui_label  : Label          = $"../UI/Label"

var _gravity           : float = 20.0
var _target_block_pos  : Vector3 = Vector3.ZERO
var _target_face_normal: Vector3 = Vector3.UP
var _looking_at_block  : bool   = false


func _ready() -> void:
	Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func _input(event: InputEvent) -> void:
	if event is InputEventMouseMotion and Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
		rotate_y(-event.relative.x * mouse_sensitivity)
		_camera.rotate_x(-event.relative.y * mouse_sensitivity)
		_camera.rotation.x = clampf(_camera.rotation.x, -deg_to_rad(89), deg_to_rad(89))

	if event is InputEventKey and event.pressed:
		match event.keycode:
			KEY_1: selected_block = _BlockTypes.GRASS
			KEY_2: selected_block = _BlockTypes.DIRT
			KEY_3: selected_block = _BlockTypes.STONE
			KEY_4: selected_block = _BlockTypes.SAND

	if event.is_action_pressed("place_block"):
		if _looking_at_block:
			_world.place_block(selected_block, _target_block_pos, _target_face_normal)

	if event.is_action_pressed("break_block"):
		if _looking_at_block:
			_world.break_block(_target_block_pos, _target_face_normal)

	if event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
		if Input.get_mouse_mode() == Input.MOUSE_MODE_CAPTURED:
			Input.set_mouse_mode(Input.MOUSE_MODE_VISIBLE)
		else:
			Input.set_mouse_mode(Input.MOUSE_MODE_CAPTURED)


func _physics_process(delta: float) -> void:
	var input_dir := Input.get_vector("move_left", "move_right", "move_forward", "move_backward")
	var direction := (transform.basis * Vector3(input_dir.x, 0, input_dir.y)).normalized()

	velocity.x = direction.x * move_speed
	velocity.z = direction.z * move_speed

	if not is_on_floor():
		velocity.y -= _gravity * delta
	elif Input.is_action_just_pressed("jump"):
		velocity.y = jump_velocity

	move_and_slide()

	_do_raycast()
	_world.update_around(global_position)
	_update_ui()


func _do_raycast() -> void:
	var origin := _camera.global_position
	var forward := -_camera.global_transform.basis.z
	var hit = _voxel_raycast(origin, forward, reach_distance)

	if hit != null:
		_looking_at_block = true
		_target_block_pos = hit["hit_pos"]
		_target_face_normal = hit["normal"]

		_highlight.visible = true
		_highlight.global_position = Vector3(
			float(hit["voxel_x"]) + 0.5,
			float(hit["voxel_y"]) + 0.5,
			float(hit["voxel_z"]) + 0.5
		)
	else:
		_looking_at_block = false
		_highlight.visible = false


func _voxel_raycast(origin: Vector3, dir: Vector3, max_dist: float) -> Variant:
	var pos := origin
	var step_dir := dir.normalized()
	var step_size := 0.08

	var t := 0.0
	var last_voxel := Vector3i(
		floori(pos.x), floori(pos.y), floori(pos.z)
	)

	while t < max_dist:
		var voxel := Vector3i(floori(pos.x), floori(pos.y), floori(pos.z))
		if voxel != last_voxel:
			var block = _world.get_block_at(voxel)
			if _BlockTypes.is_opaque(block):
				var delta := voxel - last_voxel
				var normal := Vector3.ZERO
				if delta.x != 0:
					normal = Vector3(float(-delta.x), 0, 0)
				elif delta.y != 0:
					normal = Vector3(0, float(-delta.y), 0)
				elif delta.z != 0:
					normal = Vector3(0, 0, float(-delta.z))
				return {
					"voxel_x": voxel.x, "voxel_y": voxel.y, "voxel_z": voxel.z,
					"hit_pos": pos,
					"normal": normal
				}
			last_voxel = voxel

		pos += step_dir * step_size
		t += step_size

	return null


func _update_ui() -> void:
	var block_name = _BlockTypes.name_of(selected_block)
	var chunk_n = _world.chunk_count()
	var fps = Engine.get_frames_per_second()
	_ui_label.text = "[%s] | Chunks: %d | FPS: %d | %s" % [
		block_name, chunk_n, fps,
		"Looking" if _looking_at_block else ""
	]
