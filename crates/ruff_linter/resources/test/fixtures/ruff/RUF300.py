def f():
    while cond:
        break

def f():
    while cond:
        if cond1:
            return 1
        elif cond2:
            return 2
        else:
            return 3

def f():
    while cond:
        return
    else:
        return 3

def f():
    for i in itr:
        if i>0:
            break
        else:
            return

def f():
    for i in itr:
        if i>0:
            break
        else:
            continue
    return 3
