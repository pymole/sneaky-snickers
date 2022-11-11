import pymongo
import settings


client = pymongo.MongoClient(settings.MONGO_URI)