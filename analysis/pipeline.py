import base64

import balalaika

from database import client
import settings


def load_game_log():
    db = client.get_default_database()
    collection = db.get_collection(settings.GAMES_COLLECTION_NAME)
    game = collection.find_one()
    return game


game_log = load_game_log()
print(game_log)
print(balalaika.get_positions(game_log))