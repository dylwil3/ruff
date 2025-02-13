def empty():
    pass

def single():
    x = 1

def several():
    x = 1
    foo()
    x = 2

def ifstmt():
    if cond:
        print("if")
    elif cond1:
        print("elif")
    else:
        print("else")
    print("afterif")

def implicit_else():
    if cond:
        print("if")
    print("next")

def match():
    match item:
        case 1:
            print("first")
        case 2:
            print("second")
    print("aftermatch")

def match_catchall():
    match item:
        case 1:
            print("first")
        case 2:
            print("second")
        case _:
            print("catchall")
    print("aftermatch")

def forloop():
    for i in itr:
        print("body")
    print("after")

def forloop_else():
    for i in itr:
        print("body")
    else:
        print("else")
    print("after")

def whileloop_else():
    while cond:
        print("body")
    else:
        print("else")
    print("after")

def returns():
    return 1

def breaks():
    while cond:
        break

def continues():
    while cond:
        continue

def try_except():
    try:
        print("try")
    except:
        print("except")
    print("after")

        
def try_except_handlers():
    try:
        print("try")
    except ValueError:
        print("value error")
    except TypeError:
        print("type error")
    print("after")

def try_except_else():
    try:
        print("try")
    except:
        print("except")
    else:
        print("else")
    print("after")

def try_finally():
    try:
        print("try")
    finally:
        print("finally")
    print("after")

def try_except_else_finally():
    try:
        print("try")
    except:
        print("except")
    else:
        print("else")
    finally:
        print("finally")
    print("after")


