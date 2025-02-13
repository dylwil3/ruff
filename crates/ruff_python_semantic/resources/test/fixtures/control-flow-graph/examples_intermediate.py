def simple_unreachable():
    return 1
    return 2
    print("after")

def nested_if():
    if cond0:
        print("cond0")
        if cond1:
            print("cond1")
        elif cond2:
            print("cond2")
        else:
            print("cond0 else")
    else:
        print("else")
    print("after")

def nested_loops():
    for i in itr:
        while cond:
            print("inner")
        print("outer")
    print("done")

def nested_try():
    try:
        try:
            foo()
        except:
            bar()
    except:
        buzz()
    finally:
        babble()
    print("done")

def nested_break():
    while cond:
        for i in itr:
            break
        print("outer")
        continue
    print("done")
