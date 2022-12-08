from database import client
import settings


def load_random_game_logs(n: int):
    db = client.get_default_database()
    collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
    game_logs = list(collection.aggregate([{
        "$sample": {
            "size": n
        }
    }]))
    return game_logs


def load_all_game_logs():
    db = client.get_default_database()
    collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
    game_logs = collection.find()
    return game_logs
