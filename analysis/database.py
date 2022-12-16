import json
import random
import pymongo
import os
import settings


class SelfplayRepository:
    def __init__(self, mongo_uri: str) -> None:
        self.client = pymongo.MongoClient(mongo_uri)

    def load_random_game_logs(self, n: int, tag: str = None):
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

    def get_game_log_ids(self, count: int, tag: str = None):
        db = self.client.get_default_database()
        collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
        game_logs = collection.find({"tag": tag}, projection=["_id"]).limit(count)
        ids = [str(game_log["_id"]) for game_log in game_logs]
        assert len(ids) == self.train_size + self.val_size
        return ids


class SelfplayDirectoryRepository:
    def __init__(self, directory: str) -> None:
        self.directory = directory
        self.size = len(os.listdir(self.directory))
    
    def load_random_game_logs(self, n: int, tag: str = None):
        filenames = self.get_game_log_ids(n, tag)
        game_logs = self.load_game_logs(filenames)
        return game_logs

    def load_all_game_logs(self):
        return self.load_game_logs(range(0, self.size))

    def get_game_log_ids(self, count: int, tag: str = None):
        return list(map(str, random.sample(list(range(0, self.size)), count)))

    def load_game_logs(self, ids):
        game_logs = []
        for filename in ids:
            path = os.path.join(self.directory, filename)
            with open(path, "r") as f:
                game_log = json.load(f)
            
            game_logs.append(game_log)
        
        return game_logs


def get_default_repo():
    if settings.DEFAULT_REPO == "directory":
        return SelfplayDirectoryRepository(
            settings.GAME_LOGS_DIR,
        )
    
    return SelfplayRepository(
        settings.MONGO_URI,
    )
