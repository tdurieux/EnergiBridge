import os
import sys

def run(program):
    # write program to temporary file
    with open('temp.py', 'w') as f:
        f.write(program)
    # run the temporary file
    print("running")
    import temp
    del sys.modules["temp"]
    print("done")
    # remove the temporary file
    os.remove('temp.py')
    return "success"

