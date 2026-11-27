import Base
import ./hello.bend as Hello
import ./echo.bend as Echo

# The ready line is the serial line of the smallest lerux boot.
law ready_line:
  {Hello.ready_line() == "lerux: Hello from Rust on seL4 Microkit!" : String}

# After init, a notification does not change the handler.
law handler_stays:
  for +handler: Hello.Handler
  {Hello.notified(handler) == (handler, Unit{}) : Hello.Handler & Unit}

# A notified byte is the byte the domain writes back.
law echo_byte:
  for +handler: Echo.Handler
  for +byte: U32
  {Echo.notified(handler, byte) == (handler, byte) : Echo.Handler & U32}

# The sample byte this entry writes back is the character A.
law echo_sample:
  {Echo.payload(Echo.notified(Echo.init(), Echo.sample())) == "A" : String}
