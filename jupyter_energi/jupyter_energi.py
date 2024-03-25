import subprocess
import os
import numpy as np
import pandas as pd

from extract_code import *
import matplotlib.pyplot as plt


def extract_time_and_power(dataset, cumulative=False):
    res = []
    for data in dataset:
        # get rid of header
        data = data[1:]
        # change the data to numpy array
        data = np.array(data)
        # keep only the power data and delta time
        time = data[:, 0]
        power = data[:, 27]
        # accumulate the time
        if cumulative:
            time = np.cumsum(time) / 1_000

        # make diff of power
        power = np.diff(power)
        # insert 0 at the beginning
        power = np.insert(power, 0, 0)
        if cumulative:
            power = np.cumsum(power)
            power = power[0:-1]
            time = time[0:-1]
        else:
            power = power[1:-1]
            time = time[1:-1]
        #make numpy array from time and power
        res.append(np.column_stack((time, power)))
    return res


def make_time_series_plot(time_power_dataset, cumulative=False):
    for time_power in time_power_dataset:
        time = time_power[:, 0]
        power = time_power[:, 1]
        if not cumulative:
            # multiply power usage by delta time to get energy usage
            power = power * (time / 100)
            time = np.cumsum(time) / 1000
        plt.plot(time, power)

    plt.xlabel('Time (s)')
    plt.ylabel('Power (W)')
    plt.title('Power vs Time')
    plt.show()


def make_violin_plot(time_power_dataset, cumulative=False):
    data = []
    for time_power in time_power_dataset:
        time = time_power[:, 0]
        power = time_power[:, 1]
        if not cumulative:
            # multiply power usage by delta time to get energy usage
            power = power * (time / 100)
        data.append(power)
    plt.violinplot(data)
    plt.ylabel('Power (W)')
    plt.title('Power violin plot, cumulative={cumulative}')
    plt.show()


def run(program=None, cumulative=False, no_runs=1):
    if program is None:
        extract_and_write_code(notebook_path, start_marker, end_marker)
    else:
        with open('temp.py', 'w') as f:
            f.write(program)
    # make empty list to store data
    data = []
    for i in range(no_runs):
        # run the temporary file with energibridge.exe as admin
        res = subprocess.run(['energibridge.exe', '-o', 'temp.csv', '--summary', 'py', 'temp.py'], capture_output=True,
                      text=True)
        print(res.stdout)
        print(res.stderr)
        # get data from temp.csv with pandas and append it to the dataframe
        data.append(pd.read_csv('temp.csv'))
        # remove the temporary file
        #os.remove('temp.csv')
        # get power from the data

    if program is None:
        os.remove('temp.py')
    return data


def test_run():
    data = np.genfromtxt('temp.csv', delimiter=',', skip_header=1)
    res = extract_time_and_power(data, cumulative=True)
    make_time_series_plot(res, cumulative=True)
    print(res)

