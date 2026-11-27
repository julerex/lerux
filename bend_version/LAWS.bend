import Base
import ./hello.bend as Hello

# The ready line is the serial line of the smallest lerux boot.
law ready_line:
  {Hello.ready_line() == "lerux: Hello from Rust on seL4 Microkit!" : String}

# After init, a notification does not change the handler.
law handler_stays:
  for +handler: Hello.Handler
  {Hello.notified(handler) == (handler, Unit{}) : Hello.Handler & Unit}
