while 34: # trailing test comment
    pass # trailing last statement comment

    # trailing while body comment

# leading else comment

else: # trailing else comment
    pass

    # trailing else body comment


while aVeryLongConditionThatSpillsOverToTheNextLineBecauseItIsExtremelyLongAndGoesOnAndOnAndOnAndOnAndOnAndOnAndOnAndOnAndOn: # trailing comment
    pass

else:
    pass

while (
    some_condition(unformatted, args) and anotherCondition or aThirdCondition
):  # comment
    print("Do something")


while (
    some_condition(unformatted, args) # trailing some condition
    and anotherCondition or aThirdCondition  # trailing third condition
):  # comment
    print("Do something")

while True: pass
    # leading for `else`
else:
    pass

while True:
    pass
        # trailing
else:
    pass

def foo():
    while True:
        pass
# leading
    else:
        pass
