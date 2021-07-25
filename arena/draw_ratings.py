#!/usr/bin/env python3

from matplotlib.figure import Figure
import matplotlib.pyplot as plt
import json
import sys
from math import ceil, floor
import os


def minmax(ratings):
    return [
        floor(min(r['mu'] - 2 * r['sigma'] for r in ratings.values()) / 5) * 5,
        ceil (max(r['mu'] + 2 * r['sigma'] for r in ratings.values()) / 5) * 5
    ]

def main():
    filename = 'ratings.json' if len(sys.argv) <= 1 else sys.argv[1]
    ratings = json.load(open(filename))
    if len(ratings) == 0:
        print('No ratings')
        return


    plt.style.use('classic')
    fig, ax = plt.subplots()
    boxes = [
        {
            'label' : name,
            'whislo': rating['mu'] - 3 * rating['sigma'],
            'q1'    : rating['mu'] - rating['sigma'],
            'med'   : rating['mu'],
            'q3'    : rating['mu'] + rating['sigma'],
            'whishi': rating['mu'] + 3 * rating['sigma'],
            'fliers': []
        }
        for name, rating in ratings.items()
    ]
    ax.bxp(boxes, showfliers=False)
    ax.set_ylabel("trueskill")
    ax.get_xaxis().set_tick_params(rotation=-90)
    ax.set_ylim(minmax(ratings))
    fig.tight_layout()
    fig.subplots_adjust(bottom=0.25, top=0.97)

    fig.set_size_inches(10, 10)
    plt.gca().yaxis.set_major_locator(plt.MultipleLocator(1))
    plt.gca().yaxis.set_minor_locator(plt.MultipleLocator(1))
    ax.grid()
    plt.savefig("ratings.tmp.png")
    plt.close()

    # This allows to redraw image on every ratings.json change, and watch how image updates in vscode without read errors.
    os.rename('ratings.tmp.png', 'ratings.png')


if __name__ == '__main__':
    main()
