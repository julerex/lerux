import Base

# Userspace of the serial echo protection domain: one notification
# carries a byte, and the domain writes that same byte back.
# Bend can block on a file or a socket. It cannot block on a Microkit
# notification, so this function is the byte path. It does not wait,
# and it does not pretend to be a serial device.

type Handler is Data:
  Echoing{}

def init() -> Handler:
  Echoing{}

def sample() -> U32:
  65

def notified(handler: Handler, byte: U32) -> Handler & U32:
  (handler, byte)

# Bend opens a pair only when that pair is a parameter.
def payload(parked: Handler & U32) -> String:
  (_, byte) = parked
  Char.show(Char.from_u32(byte))
