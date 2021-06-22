import json
import os
import dash
import dash_core_components as dcc
import dash_html_components as html
from dash.dependencies import Input, Output
import plotly.graph_objects as go
import moves

GAMES_FOLDER = 'data/'
FILES = []


def update_files():
    global FILES
    FILES = os.listdir(GAMES_FOLDER)


update_files()


app = dash.Dash(__name__)

app.layout = html.Div([
    dcc.Dropdown(
        id="game-file-dropdown",
        options=[{'label': name, 'value': name} for name in FILES],
    ),
    dcc.Graph(id="snake-metrics"),
    dcc.Link(id='game-url', href='/'),
])


@app.callback(
    [Output("snake-metrics", "figure"), Output("game-url", "href")],
    [Input("game-file-dropdown", "value")])
def load_game_file(game_file_name):
    with open(f'data/{game_file_name}', 'r') as f:
        game = json.load(f)

    colors = {snake['Name']: snake['Color'] for snake in game['Frames'][0]['Snakes']}

    metrics = moves.analize(game)

    fig = go.Figure()

    for name, values in metrics.items():
        fig.add_trace(go.Scatter(x=list(range(len(values))), y=values, line=dict(color=colors[name]), name=name))

    fig.update_layout(
        xaxis_title="Turns",
        yaxis_title="Metric",
        legend_title="Snakes",
    )
    return fig, f'https://play.battlesnake.com/g/{os.path.splitext(game_file_name)[0]}/'


if __name__ == '__main__':
    app.run_server(debug=True)
