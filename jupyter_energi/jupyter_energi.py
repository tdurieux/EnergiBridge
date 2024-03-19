import os
import sys
import subprocess


def run(program):
    # write program to temporary file
    with open('temp.py', 'w') as f:
        f.write(program)
    # run the temporary file with energibridge.exe

#    res = subprocess.run(['energibridge.exe', '-o', 'temp.csv', '--summary', 'py', 'temp.py'], capture_output=True, text=True)
    res = os.system(r'''
    Powershell -Command "& { Start-Process \"energibridge.exe\"
     -ArgumentList @("-o", "temp.csv", "--summary", "py", "temp.py")
     -Verb RunAs } " ''')
    print(res)
    # remove the temporary file
    os.remove('temp.py')
    return "success"

