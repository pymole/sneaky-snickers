import os

MONGO_URI = os.getenv("MONGO_URI", "mongodb://battlesnake:battlesnake@localhost:27017/battlesnake?authSource=admin")
GAMES_COLLECTION_NAME = os.getenv("GAMES_COLLECTION_NAME", "games")
SNAKES_COUNT = 4
WIDTH = 11
HEIGHT = 11