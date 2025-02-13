def complicated_control_flow():
    result = []
    outer_counter = 0
    continue_loop = True

    while continue_loop:
        # Case 1: When outer_counter is divisible by 4
        if outer_counter % 4 == 0:
            inner_counter = outer_counter
            # A nested while loop with its own control flow
            while inner_counter < outer_counter + 5:
                if inner_counter % 2 == 0:
                    result.append(f"Outer {outer_counter}: Even inner {inner_counter}")
                else:
                    result.append(f"Outer {outer_counter}: Odd inner {inner_counter}")
                inner_counter += 1

        # Case 2: When outer_counter gives remainder 1 modulo 4
        elif outer_counter % 4 == 1:
            # A for loop with try/except handling
            for i in range(3):
                try:
                    # Avoid division by zero using a fallback value
                    divisor = i if i != 0 else 1
                    computed_value = (outer_counter ** 2) / divisor
                    if computed_value > 10:
                        result.append(f"Outer {outer_counter}, i {i}: High value {computed_value}")
                    else:
                        result.append(f"Outer {outer_counter}, i {i}: Low value {computed_value}")
                except Exception as e:
                    result.append(f"Outer {outer_counter}, i {i}: Error {str(e)}")

        # Case 3: All other cases
        else:
            nested_flag = False
            # A nested while loop with internal conditional adjustments
            while not nested_flag:
                result.append(f"Nested loop at outer {outer_counter}")
                outer_counter += 1  # Modify outer_counter within nested loop
                # Exit condition based on the updated outer_counter
                if outer_counter % 3 == 0 or outer_counter > 20:
                    nested_flag = True
                    # Optionally break out of the main loop if outer_counter is high enough
                    if outer_counter > 20:
                        continue_loop = False

        outer_counter += 1
        # Terminate the main loop if outer_counter exceeds a threshold
        if outer_counter > 15:
            continue_loop = False

    return result

def nested_try_except_finally_example():
    output = []
    for i in range(10):
        output.append(f"Iteration {i} start")
        try:
            output.append("Outer try block started")
            try:
                output.append("  Inner try block started")
                if i == 1:
                    output.append("  i==1: triggering continue (from inner try)")
                    # The inner finally (and then outer finally) will still run before continuing.
                    continue
                elif i == 2:
                    output.append("  i==2: raising RuntimeError to be caught in inner except")
                    raise RuntimeError("Error triggered at i==2")
                elif i == 3:
                    output.append("  i==3: processing normally in inner try")
                elif i == 4:
                    output.append("  i==4: entering nested try block")
                    try:
                        output.append("    Nested try block started")
                        # For demonstration, raise a KeyError in the nested try
                        if i % 2 == 0:
                            output.append("    Nested try: raising KeyError")
                            raise KeyError("Nested error at i==4")
                    except KeyError as ke:
                        output.append(f"    Nested except caught: {ke}")
                        # Use continue here to jump to the next iteration of the outer loop.
                        continue
                    finally:
                        output.append("    Nested finally executed")
                elif i == 5:
                    output.append("  i==5: triggering return from inner try")
                    return output  # Return jump out of the function.
                elif i == 6:
                    output.append("  i==6: triggering break from inner try")
                    break  # Break out of the for-loop.
                else:
                    output.append(f"  Processing i=={i} normally in inner try")
                output.append("  Inner try block finished normally")
            except RuntimeError as re:
                output.append(f"  Inner except caught RuntimeError: {re}")
                # Continue to the next iteration of the for-loop after handling the error.
                continue
            finally:
                output.append("  Inner finally executed")
            output.append("Outer try block finished normally")
        except Exception as e:
            output.append(f"Outer except caught: {e}")
        finally:
            output.append("Outer finally executed")
        output.append(f"Iteration {i} end")
    output.append("Loop finished")
    return output



