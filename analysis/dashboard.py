#!/usr/bin/env python3
import dataclasses
import json
import os
from pathlib import Path

import dash
import dash_core_components as dcc
import dash_html_components as html
from dash.dependencies import Input, Output, State
import metrics
import scrape
import match
import dacite


STORAGE = Path(os.environ.get('STORAGE', 'data/'))
METRICS = {
    'flavored_flood_fill': metrics.FlavoredFloodFill,
    'move_availability': metrics.MovesAvailability,
    'flood_fill': metrics.FloodFill,
}

GAME_FILES = os.listdir(STORAGE)


external_stylesheets = ['https://codepen.io/chriddyp/pen/bWLwgP.css']
app = dash.Dash(__name__, external_stylesheets=external_stylesheets)


app.layout = html.Div([
    html.H3(
        children=dcc.Link(id='game-url', href='/'),
        style={
            'textAlign': 'center',
        }
    ),
    html.Div(
        [
            dcc.RadioItems(
                id='search-type-radio',
                options=[
                    {'label': 'Search files', 'value': 'file'},
                    {'label': 'Game ID', 'value': 'id'},
                ],
                value='file',
                style={'width': '40%'},
            ),
            html.Div(
                [
                    dcc.Dropdown(
                        id="game-file-dropdown",
                        options=[{'label': name, 'value': name} for name in GAME_FILES],
                        value=GAME_FILES[0],
                        style={
                            'width': '100%',
                            'display': 'inline-block'
                        },
                    ),
                    dcc.Input(
                        id='game-id-input',
                        disabled=True,
                    ),
                ],
                style={'width': '100%'},
            ),
            dcc.Dropdown(
                id="metric-dropdown",
                options=[{'label': name, 'value': name} for name in METRICS.keys()],
                value=next(iter(METRICS.keys())),
                style={
                    'width': '100%',
                    'display': 'inline-block'
                }
            ),
            html.Button('Analyze', id='analyze-button', style={'width': '100%'}),
        ],
        style={
            'display': 'flex',
            'flex-direction': 'row',
            'justify-content': 'space-evenly',
            'padding-top': '16px'
        }
    ),
    html.Div(id='metric-dashboard'),
])


@app.callback(
    [Output('game-file-dropdown', 'disabled'), Output('game-id-input', 'disabled')],
    [Input('search-type-radio', 'value')])
def change_type(search_type):
    if search_type == 'file':
        return False, True

    return True, False


@app.callback(
    [Output('metric-dashboard', 'children'), Output('game-url', 'href')],
    [
        Input('analyze-button', 'n_clicks'),
        State('game-file-dropdown', 'value'),
        State('game-id-input', 'value'),
        State('metric-dropdown', 'value'),
        State('search-type-radio', 'value'),
    ])
def estimate(button_clicks, game_file_name, game_id, metric, search_type):
    global GAME_FILES
    GAME_FILES = os.listdir(STORAGE)

    if search_type == 'id':
        game_file_name = search_type + '.json'

    if game_file_name not in GAME_FILES:
        snickers_match = match.battlesnake_frames_to_snickers_match(scrape.download(game_id))
        scrape.save_game(STORAGE, game_id, dataclasses.asdict(snickers_match))
    else:
        with open(f'{STORAGE}/{game_file_name}', 'r') as f:
            data = json.load(f)
            try:
                snickers_match = dacite.from_dict(match.SnickersMatch, data)
            except:
                snickers_match = match.battlesnake_frames_to_snickers_match(data)

        game_id = os.path.splitext(game_file_name)[0]

    figures = METRICS[metric](snickers_match).analyze()
    metric_dashboard = [
        dcc.Graph(figure=figure)
        for figure in figures
    ]

    return metric_dashboard, f'https://play.battlesnake.com/g/{game_id}/'


if __name__ == '__main__':
    app.run_server(debug=True)
