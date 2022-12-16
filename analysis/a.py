import pymongo
import torch

import database as db
import settings

# # sp = db.SelfplayRepository(settings.MONGO_URI)

# client = pymongo.MongoClient(settings.MONGO_URI)
# db = client.get_default_database()

# collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
# # collection.insert_many(game_logs)
# res = collection.update_many({"tag": None}, {"$set": {"tag": "poc"}})
# print(res.matched_count)

print(len(torch.Tensor([1, 2, 3])))