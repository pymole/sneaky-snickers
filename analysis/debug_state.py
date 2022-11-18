import json

import balalaika


with open('../balalaika/src/debug_data/suicide2/turn2.json', 'r') as f:
    state = json.load(f)

board = balalaika.get_board_from_state(state)
actions = balalaika.search(board, 100)
print(actions)

balalaika.draw_board(board)
flood_fill = balalaika.flood_fill(board)
print(flood_fill)
print(balalaika.get_masks(board))

board = balalaika.advance_one_step(board, actions)
balalaika.draw_board(board)
flood_fill = balalaika.flood_fill(board)
print(flood_fill)