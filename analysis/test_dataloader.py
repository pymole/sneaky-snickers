import dataset
import time
import balalaika
import settings
import database


def test_get_features():
    repo = database.get_default_repo()
    (game_log,) = repo.load_random_game_logs(1)
    _, boards, _ = balalaika.rewind(game_log)
    balalaika.draw_board(boards[10])
    print(balalaika.get_features(boards[10], feature_set_tags=["flood_fill"]))


def test_dataloader():
    feature_set_sizes = balalaika.get_feature_set_sizes()
    print(feature_set_sizes)

    repo = database.get_default_repo()
    game_log_ids = repo.get_game_log_ids(tag='poc', count=1)

    dataloader = balalaika.DataLoader(
        mongo_uri=settings.MONGO_URI,
        batch_size=256,
        prefetch_batches=20,
        mixer_size=150000,
        feature_set_tags=["flood_fill", "global_metrics", "snakes_metrics"],
        game_log_ids=game_log_ids,
        random_batch=True,
    )

    summa = 0.0
    count = 0

    for i in range(1):
        print(i)
        start = time.time()
        batch = next(dataloader)

        for (indices, values), rewards in batch:
            print(indices)
            print(values)
            print(rewards)
            assert len(indices) == len(set(indices))

        summa += time.time() - start
        count += 1

    print("20_000 batches (batch_size=256). Total:", summa, "Average:", summa / float(count))

test_get_features()
test_dataloader()