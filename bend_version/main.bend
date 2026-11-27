import Base
import ./hello.bend as Hello

# Language entry. `bend bend_version/main.bend` runs this main.
# It brings up the hello protection domain, prints the ready line,
# and runs the idle notification handler once. There is no kernel
# left to wait on after that.

def main() -> IO(Unit):
  do IO<Unit>:
    handler : Hello.Handler <- Hello.init()
    return Hello.settle(handler)
