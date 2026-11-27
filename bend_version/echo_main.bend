import Base
import ./echo.bend as Echo

# Entry for the serial echo protection domain.
# `bend bend_version/echo_main.bend` runs one notified byte and writes
# that byte back. The hello entry is still `main.bend`.

def main() -> IO(Unit):
  do IO<Unit>:
    handler : Echo.Handler = Echo.init()
    parked : Echo.Handler & U32 = Echo.notified(handler, Echo.sample())
    IO.print(Echo.payload(parked))
