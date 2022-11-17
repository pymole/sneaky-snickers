import pipeline
import balalaika
from settings import WIDTH, HEIGHT


def are_close_points(p1, p2) -> bool:
    diff_x = abs(p1['x'] - p2['x'])
    diff_y = abs(p1['y'] - p2['y'])
    return (
        diff_x <= 1 and diff_y <= 1 or
        diff_x == WIDTH - 1 and diff_y <= 1 or
        diff_x <= 1 and diff_y == HEIGHT - 1 or
        diff_x == WIDTH - 1 and diff_y == WIDTH - 1
    )


def is_head_to_head_board(board):
    snakes = board['snakes']

    heads = []

    for snake in snakes:
        cur_head = snake['body'][0]
        for head in heads:
            if are_close_points(cur_head, head):
                return True

        heads.append(cur_head)

    return False



game_logs = pipeline.load_random_game_logs(399)
head_to_head_total = 0
turns_total = 0
for game_log in game_logs:
    turns_total += game_log['turns']

    _, boards, (rewards, _) = balalaika.rewind(game_log)

    for board in boards:
        if is_head_to_head_board(board):
            head_to_head_total += 1
            # balalaika.draw_board(board)
            # input()


n = len(game_logs)
avg_head_to_head = head_to_head_total / n
avg_turns = turns_total / n

print(f"Average head to head positions count in {n} game logs: {avg_head_to_head}")
print(f"Average turns count in {n} game logs: {avg_turns}")
