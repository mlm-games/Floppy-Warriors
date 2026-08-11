extends Control

var shop_overlay: Control
var bones_label: Label
var stats_label: Label


func _ready() -> void:
	_build_hud()
	_refresh_labels()
	%PlayButton.grab_focus()

	if get_tree().has_meta("offline_bones"):
		var offline := int(get_tree().get_meta("offline_bones"))
		get_tree().remove_meta("offline_bones")
		if offline > 0:
			_toast("While you were away: +%d Bones" % offline)

	if not Meta.bones_changed.is_connected(_on_bones_changed):
		Meta.bones_changed.connect(_on_bones_changed)


func _build_hud() -> void:
	# Top-left stats
	var margin := MarginContainer.new()
	margin.set_anchors_preset(Control.PRESET_TOP_LEFT)
	margin.add_theme_constant_override("margin_left", 24)
	margin.add_theme_constant_override("margin_top", 24)
	add_child(margin)

	var vbox := VBoxContainer.new()
	margin.add_child(vbox)

	bones_label = Label.new()
	bones_label.add_theme_font_size_override("font_size", 28)
	vbox.add_child(bones_label)

	stats_label = Label.new()
	stats_label.add_theme_font_size_override("font_size", 18)
	vbox.add_child(stats_label)

	# Bottom buttons row under Play
	var bottom := HBoxContainer.new()
	bottom.set_anchors_preset(Control.PRESET_CENTER_BOTTOM)
	bottom.offset_top = -120
	bottom.offset_bottom = -40
	bottom.offset_left = -220
	bottom.offset_right = 220
	bottom.alignment = BoxContainer.ALIGNMENT_CENTER
	bottom.add_theme_constant_override("separation", 16)
	add_child(bottom)

	var shop_btn := Button.new()
	shop_btn.text = "Bone Shop"
	shop_btn.custom_minimum_size = Vector2(160, 48)
	shop_btn.pressed.connect(_open_shop)
	bottom.add_child(shop_btn)

	var settings_btn := Button.new()
	settings_btn.text = "Settings"
	settings_btn.custom_minimum_size = Vector2(160, 48)
	settings_btn.pressed.connect(_on_settings_pressed)
	bottom.add_child(settings_btn)


func _refresh_labels() -> void:
	if is_instance_valid(bones_label):
		bones_label.text = "Bones: %d" % Meta.bones
	if is_instance_valid(stats_label):
		stats_label.text = (
			"Best round: %d   |   Runs: %d   |   Wins: %d"
			% [Meta.best_round, Meta.total_runs, Meta.total_victories]
		)


func _on_bones_changed(_amount: int) -> void:
	_refresh_labels()


func _on_play_button_pressed() -> void:
	STransitions.change_scene_with_transition(C.SCREENS.GAME)


func _on_settings_pressed() -> void:
	STransitions.change_scene_with_transition(C.SCREENS.SETTINGS)


func _open_shop() -> void:
	if is_instance_valid(shop_overlay):
		return

	var bg := ColorRect.new()
	bg.color = Color(0.02, 0.03, 0.05, 0.92)
	bg.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.mouse_filter = Control.MOUSE_FILTER_STOP
	add_child(bg)
	shop_overlay = bg

	var center := CenterContainer.new()
	center.set_anchors_preset(Control.PRESET_FULL_RECT)
	bg.add_child(center)

	var panel := PanelContainer.new()
	panel.custom_minimum_size = Vector2(720, 520)
	center.add_child(panel)

	var root := VBoxContainer.new()
	root.add_theme_constant_override("separation", 12)
	panel.add_child(root)

	var title := Label.new()
	title.text = "Bone Shop"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.add_theme_font_size_override("font_size", 32)
	root.add_child(title)

	var bal := Label.new()
	bal.name = "BalanceLabel"
	bal.text = "Bones: %d" % Meta.bones
	bal.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	root.add_child(bal)

	var scroll := ScrollContainer.new()
	scroll.size_flags_vertical = Control.SIZE_EXPAND_FILL
	scroll.custom_minimum_size = Vector2(0, 360)
	root.add_child(scroll)

	var list := VBoxContainer.new()
	list.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	list.add_theme_constant_override("separation", 8)
	scroll.add_child(list)

	for entry in Meta.CATALOG:
		list.add_child(_make_shop_row(entry, bal))

	var close_btn := Button.new()
	close_btn.text = "Close"
	close_btn.pressed.connect(_close_shop)
	root.add_child(close_btn)
	close_btn.grab_focus()


func _make_shop_row(entry: Dictionary, balance_label: Label) -> Control:
	var id := StringName(entry["id"])
	var row := HBoxContainer.new()
	row.add_theme_constant_override("separation", 12)

	var info := VBoxContainer.new()
	info.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	row.add_child(info)

	var name_l := Label.new()
	var lvl := Meta.get_level(id)
	var max_l := int(entry["max_level"])
	name_l.text = "%s  (%d/%d)" % [entry["display_name"], lvl, max_l]
	info.add_child(name_l)

	var desc := Label.new()
	desc.text = str(entry["description"])
	desc.modulate = Color(0.8, 0.85, 0.9)
	info.add_child(desc)

	var buy_btn := Button.new()
	buy_btn.custom_minimum_size = Vector2(140, 48)
	if lvl >= max_l:
		buy_btn.text = "MAX"
		buy_btn.disabled = true
	else:
		var cost := Meta.get_cost(id)
		buy_btn.text = "%d Bones" % cost
		buy_btn.disabled = not Meta.can_buy(id)
		buy_btn.pressed.connect(
			func() -> void:
				if Meta.buy(id):
					_close_shop()
					_open_shop()
					_refresh_labels()
		)
	row.add_child(buy_btn)
	return row


func _close_shop() -> void:
	if is_instance_valid(shop_overlay):
		shop_overlay.queue_free()
	shop_overlay = null
	%PlayButton.grab_focus()


func _toast(message: String) -> void:
	var label := Label.new()
	label.text = message
	label.add_theme_font_size_override("font_size", 22)
	label.set_anchors_preset(Control.PRESET_CENTER_TOP)
	label.offset_top = 80
	label.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	add_child(label)
	var tw := create_tween()
	tw.tween_interval(2.5)
	tw.tween_property(label, "modulate:a", 0.0, 0.6)
	tw.tween_callback(label.queue_free)