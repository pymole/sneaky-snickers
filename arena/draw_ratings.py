#!/usr/bin/env python3

from matplotlib.figure import Figure
import matplotlib.pyplot as plt
import json
import sys


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
    ax.set_ylim([20, 40])
    fig.tight_layout()
    fig.subplots_adjust(bottom=0.12, top=0.97)

    fig.set_size_inches(20, 15)
    plt.gca().yaxis.set_major_locator(plt.MultipleLocator(5))
    plt.gca().yaxis.set_minor_locator(plt.MultipleLocator(1))
    ax.grid()
    plt.savefig("ratings.png")
    plt.close()


if __name__ == '__main__':
    main()
