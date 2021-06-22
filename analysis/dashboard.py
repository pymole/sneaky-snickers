import time
import argparse
import json
from typing import Callable
from matplotlib.animation import FuncAnimation
from matplotlib.lines import Line2D

import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator


def realtime_readline(file):
    with open(file) as f:
        while True:
            line = f.readline()
            if line:
                yield line



class Dashboard:
    def __init__(self, log_path, metric: Callable):
        self.log_path = log_path
        self.metric = metric
        
        self.game_id = None
        self.turn = 0

        plt.ion()
        self.figure = plt.figure()
        
        ax = self.figure.add_subplot(xlabel='Turn', ylabel='Metric value')
        ax.legend(title='Snakes')
        ax.grid(True)
        ax.xaxis.set_major_locator(MaxNLocator(20, integer=True))
        ax.set_ylim(0, 1)
        self.ax = ax

        self.snake_lines: dict[str, Line2D] = {}
        self.snake_metrics: dict[str, float] = {}
    
    def start(self):

        for line in realtime_readline(self.log_path):
            tokens = line.split(' - ')
            if len(tokens) != 3:
                continue
            
            _, command, data = tokens
            try:
                data = json.loads(data)
            except:
                print("JSON ERROR")
                continue

            if self.game_id == data['game']['id']:
                if command == 'MOVE':
                    self.turn = data['turn']
                    
                    for name, metric in self.metric(data):
                        line = self.snake_lines[name]
                        (x, y) = self.snake_metrics[name]
                        x.append(self.turn)
                        y.append(metric)
                        snake_data = (x, y)
                        self.snake_metrics[name] = snake_data
                        line.set_data(snake_data)
                        
            
                elif command == 'END':
                    self.game_id = None
            
            elif command == 'START':
                self.ax.lines.clear()
                self.snake_lines.clear()

                self.game_id = data['game']['id']
                self.turn = data['turn']
                for name, metric in self.metric(data):
                    snake_data = ([self.turn], [metric])
                    self.snake_metrics[name] = snake_data
                    line, = self.ax.plot(*snake_data, label=name)
                    self.snake_lines[name] = line

            else:
                continue
            time.sleep(0.1)
            print('sosi')
            self.figure.canvas.draw()
            self.figure.canvas.flush_events()


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('log', type=str)
    args = parser.parse_args()

    Dashboard(args.log, lambda x: [(snake['name'], snake['health']) for snake in x['board']['snakes']]).start()
