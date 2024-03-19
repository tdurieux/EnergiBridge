import subprocess
import os
import numpy as np


def extract_power(data):
    total_res = []

    power = data[:, 26]
    delta = data[:, 0]
    res = 0
    for i in range(0, len(power)):
        res += power[i] * (delta[i] / 1_000)
    print(res)
    total_res.append(res)

    return total_res


def run(program):
    with open('temp.py', 'w') as f:
        f.write(program)
    # run the temporary file with energibridge.exe as admin
    res = subprocess.run(['energibridge.exe', '-o', 'temp.csv', '--summary', 'py', 'temp.py'], capture_output=True,
                  text=True)
    print(res.stdout)
    print(res.stderr)
    # remove the temporary file
    os.remove('temp.py')
    # get data from temp.csv with numpy and skip the first line
    data = np.genfromtxt('temp.csv', delimiter=',', skip_header=1)
    # remove the temporary file
    #os.remove('temp.csv')
    # get power from the data
    power = extract_power(data)

    return power



