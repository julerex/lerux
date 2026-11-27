import Base

# Userspace of the smallest lerux boot: one protection domain whose init
# prints the ready line and whose notification handler is idle.
# Bend has no Security Enhanced L4 kernel and no Microkit runtime, so
# this file is that program. It does not schedule, map, or trap.

type Handler is Data:
  Booted{}

def ready_line() -> String:
  "lerux: Hello from Rust on seL4 Microkit!"

def init() -> IO(Handler):
  do IO<Handler>:
    IO.print(ready_line())
    return Booted{}

def notified(handler: Handler) -> Handler & Unit:
  (handler, Unit{})

# Bend opens a pair only when that pair is a parameter.
def unit_of(parked: Handler & Unit) -> Unit:
  (_, done) = parked
  done

def settle(handler: Handler) -> Unit:
  unit_of(notified(handler))
