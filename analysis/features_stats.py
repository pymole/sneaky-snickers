import balalaika
import pipeline

game_logs = pipeline.load_random_game_logs(399)

set_max = 0
set_min = float('inf')
set_total = 0
boards_count = 0

removed_total = 0
added_total = 0
changed_total = 0
removed_max = 0
removed_min = float('inf')
added_max = 0
added_min = float('inf')
changed_max = 0
changed_min = float('inf')

for game_log in game_logs:
    _, boards, _ = balalaika.rewind(game_log)
    boards = boards[:-1]
    boards_count += len(boards)

    last_features = None

    for board in boards:
        features: list[bool] = balalaika.get_nnue_features(board)
        set_count = features.count(True)

        set_total += set_count
        set_max = max(set_max, set_count)
        set_min = min(set_min, set_count)

        if last_features is not None:
            removed = 0
            added = 0
            
            for last_feature, feature in zip(last_features, features):
                if last_feature and not feature:
                    removed += 1
                elif not last_feature and feature:
                    added += 1
            
            changed = added + removed

            removed_total += removed
            added_total += added
            changed_total += changed
            removed_max = max(removed_max, removed)
            removed_min = min(removed_min, removed)
            added_max = max(added_max, added)
            added_min = min(added_min, added)
            changed_max = max(changed_max, changed)
            changed_min = min(changed_min, changed)


        last_features = features


print(f"Features count: {len(features)}")
print(f"Minimum features set: {set_min}")
print(f"Maximum features set: {set_max}")
print(f"Average sparsity: {set_total / len(features) / boards_count:.2%}".format())

boards_count_except_first = boards_count - len(game_logs)
print("\nFeatures on move")
print(f"Added: min {added_min}; avg {added_total / boards_count_except_first:.0f}; max {added_max}")
print(f"Removed: min {removed_min}; avg {removed_total / boards_count_except_first:.0f}; max {removed_max}")
print(f"Changed: min {changed_min}; avg {changed_total / boards_count_except_first:.0f}; max {changed_max}")
