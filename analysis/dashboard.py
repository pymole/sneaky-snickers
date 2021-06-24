import json
import os
import dash
import dash_core_components as dcc
import dash_html_components as html
from dash.dependencies import Input, Output, State
import plotly.graph_objects as go
import metrics

GAMES_FOLDER = os.environ.get('DATA_FOLDER', 'data/')
METRICS = {
    'move_availability': metrics.MovesAvailability,
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
    dcc.Graph(id="snake-metrics"),
])


def get_figure(game_file_name, metric):
    with open(f'data/{game_file_name}', 'r') as f:
        game = json.load(f)

    colors = {snake['Name']: snake['Color'] for snake in game['Frames'][0]['Snakes']}

    fig = go.Figure()

    for name, values in METRICS[metric](game).analyze().items():
        fig.add_trace(go.Scatter(x=list(range(len(values))), y=values, line=dict(color=colors[name]), name=name))

    fig.update_layout(
        xaxis_title="Turn",
        yaxis_title="State estimate",
        legend_title="Snakes",
    )

    return fig, f'https://play.battlesnake.com/g/{os.path.splitext(game_file_name)[0]}/'


@app.callback(
    [Output("snake-metrics", "figure"), Output("game-url", "href")],
    [Input('analyze-button', 'n_clicks'),
    State("game-file-dropdown", "value"),
    State("metric-dropdown", 'value')])
def load_game_file(button, game_file_name, metric):
    return get_figure(game_file_name, metric)


if __name__ == '__main__':
    app.run_server(debug=True)
