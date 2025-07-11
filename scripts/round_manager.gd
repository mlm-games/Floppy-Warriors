extends Node

signal ranking_points_updated(amount: int)

var ranking_points: int = 0:
	set(val):
		ranking_points = val
		ranking_points_updated.emit(ranking_points)

var player: Player

func reset():
	player = get_tree().get_first_node_in_group("Player")
