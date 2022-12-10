# import balalaika
# import pipeline


# (game_log,) = pipeline.load_random_game_logs(1)
# _, boards, _ = balalaika.rewind(game_log)
# board = boards[100]
# balalaika.draw_board(board)
# print(len(balalaika.get_examples(board)))


import time
import balalaika
import settings

# TODO: examples_size is bad name for buffer
dataloader = balalaika.DataLoader(
    mongo_uri=settings.MONGO_URI,
    batch_size=256,
    batches_in_advance=20,
    examples_size=150000,
)

summa = 0.0
count = 0

for i in range(20_000):
    print(i)
    start = time.time()
    batch = next(dataloader)

    summa += time.time() - start
    count += 1

print("20_000 batches (batch_size=256). Total:", summa, "Average:", summa / float(count))