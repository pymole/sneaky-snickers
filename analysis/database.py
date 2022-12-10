import pymongo
import settings


class SelfplayRepository:
    def __init__(self, mongo_uri: str) -> None:
        self.client = pymongo.MongoClient(mongo_uri)

    def load_random_game_logs(self, n: int):
        db = self.client.get_default_database()
        collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
        game_logs = list(collection.aggregate([{
            "$sample": {
                "size": n
            }
        }]))
        return game_logs


    def load_all_game_logs(self):
        db = self.client.get_default_database()
        collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
        game_logs = collection.find()
        return game_logs

    def get_game_log_ids(self, tag: str, count: int):
        db = self.client.get_default_database()
        collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
        game_logs = collection.find({"tag": tag}, projection=["_id"]).limit(count)
        ids = [str(game_log["_id"]) for game_log in game_logs]
        return ids
