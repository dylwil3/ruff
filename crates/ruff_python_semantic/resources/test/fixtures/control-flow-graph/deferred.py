def func():
    try:
        return 2
    finally:
        print("hey!")
    x = 1
    return 7

def func():
    try:
        return 2
    finally:
        print("hey!")
        return 3
    x = 1
    return 7

def func():
    while True:
        try:
            break
        finally:
            continue
        print("out of the try")
    print("out of the loop")

