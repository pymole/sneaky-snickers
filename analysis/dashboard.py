#!/usr/bin/env python3

import json
import os
import dash
import dash_core_components as dcc
import dash_html_components as html
from dash.dependencies import Input, Output, State
import metrics

GAMES_FOLDER = os.environ.get('DATA_FOLDER', 'data/')
METRICS = {
    'move_availability': metrics.MovesAvailability,
    'flood_fill': metrics.FloodFill,
}
GAME_FILES = os.listdir(GAMES_FOLDER)


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
            dcc.Dropdown(
                id="game-file-dropdown",
                options=[{'label': name, 'value': name} for name in GAME_FILES],
                value=GAME_FILES[0],
                style={
                    'width': '100%',
                    'display': 'inline-block'
                }
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
            html.Button('Analyze', id='analyze-button', style={'width': '20%'}),
        ],
        style={
            'display': 'flex',
            'flex-direction': 'row',
            'justify-content': 'center',
            'padding-top': '16px'
        }
    ),
    html.Div(id='metric-dashboard'),
])


@app.callback(
    [Output('metric-dashboard', 'children'), Output('game-url', 'href')],
    [Input('analyze-button', 'n_clicks'),
    State('game-file-dropdown', 'value'),
    State('metric-dropdown', 'value')])
def load_game_file(button_clicks, game_file_name, metric):
    with open(f'data/{game_file_name}', 'r') as f:
        game = json.load(f)

    figures = METRICS[metric](game).analyze()
    metric_dashboard = [
        dcc.Graph(figure=figure)
        for figure in figures
    ]

    return metric_dashboard, f'https://play.battlesnake.com/g/{os.path.splitext(game_file_name)[0]}/'


if __name__ == '__main__':
    app.run_server(debug=True)
