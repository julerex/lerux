----------------------------- MODULE PosixSocket -----------------------------
(***************************************************************************
The one-socket table and the first file descriptor. One step is one branch.
This module does not execute Rust. The replay calls the real functions.

The socket table is userspace/crates/lerux-posix/src/socket.rs:

  Tcp::listen default           64-66     errno 95
  Sockets::new                  103-109   the stored number is 3
  socket                        116-122   a second socket is errno 24
  connect                       133-145   a second connect is errno 106
  listen                        153-161   an idle socket becomes listening
  accept                        170-177   waits in wait_until_open
  send                          186-209   open() must succeed first
  shutdown                      221-242   phase Closed, the entry stays
  close                         249-257   phase Empty
  open                          321-328   anything except Open is errno 107

The file table is userspace/crates/lerux-posix/src/lib.rs:

  MAX_FDS and TABLE_LEN         37-39     three numbers are not stored
  open                          261-277   the first success is index + 3
  idle                          560-565   a command in flight is errno 16
  exchange                      625-627   the same guard
  index                         664-666   fd.0 - 3

PeerArrives is the pipe's poll reporting ready. accept stays disabled until
that happens. No action stores descriptor 0, 1, or 2.

What this leaves out is listed in
.cursor/skills/verify-posix-socket/SKILL.md.
***************************************************************************)

EXTENDS Integers

CONSTANTS
  FirstFd, SecondSocket, ListenUnsupported, ListenInUse,
  SecondConnect, SendBefore, Busy, StdioStored

ASSUME FirstFd = 3
ASSUME SecondSocket = 24
ASSUME ListenUnsupported = 95
ASSUME ListenInUse = 98
ASSUME SecondConnect = 106
ASSUME SendBefore = 107
ASSUME Busy = 16
ASSUME StdioStored = FALSE

VARIABLES
  phase, fd, peer,
  fileOpen, fileFlight,
  lastErrno, lastOp,
  stored0, stored1, stored2

vars == <<
  phase, fd, peer,
  fileOpen, fileFlight,
  lastErrno, lastOp,
  stored0, stored1, stored2
>>

fileVars == <<fileOpen, fileFlight, stored0, stored1, stored2>>
socketVars == <<phase, fd, peer>>

Init ==
  /\ phase = "empty"
  /\ fd = 0
  /\ peer = FALSE
  /\ fileOpen = FALSE
  /\ fileFlight = FALSE
  /\ lastErrno = 0
  /\ lastOp = "init"
  /\ stored0 = FALSE
  /\ stored1 = FALSE
  /\ stored2 = FALSE

\* The first socket is 3. Any later socket, while the entry is held, is 24.
Socket ==
  /\ IF phase = "empty"
     THEN /\ phase' = "idle"
          /\ fd' = FirstFd
          /\ peer' = FALSE
          /\ lastErrno' = 0
     ELSE /\ lastErrno' = SecondSocket
          /\ UNCHANGED socketVars
  /\ lastOp' = "socket"
  /\ UNCHANGED fileVars

\* listen on an idle socket binds the port and keeps this same entry.
ListenOk ==
  /\ phase = "idle"
  /\ phase' = "listening"
  /\ fd' = fd
  /\ peer' = FALSE
  /\ lastErrno' = 0
  /\ lastOp' = "listen"
  /\ UNCHANGED fileVars

\* The trait default refuses to listen.
ListenDefault ==
  /\ phase = "idle"
  /\ lastErrno' = ListenUnsupported
  /\ lastOp' = "listenDefault"
  /\ UNCHANGED <<socketVars, fileVars>>

\* The pipe rejects the port as already bound. The entry stays idle.
ListenBound ==
  /\ phase = "idle"
  /\ lastErrno' = ListenInUse
  /\ lastOp' = "listenBound"
  /\ UNCHANGED <<socketVars, fileVars>>

\* The peer is connected. accept is still a separate step.
PeerArrives ==
  /\ phase = "listening"
  /\ ~peer
  /\ peer' = TRUE
  /\ lastErrno' = 0
  /\ lastOp' = "peer"
  /\ UNCHANGED <<phase, fd, fileVars>>

\* Enabled only once the peer is connected. The same entry becomes open.
Accept ==
  /\ phase = "listening"
  /\ peer
  /\ phase' = "open"
  /\ fd' = fd
  /\ peer' = TRUE
  /\ lastErrno' = 0
  /\ lastOp' = "accept"
  /\ UNCHANGED fileVars

\* The first connect opens the one socket.
Connect ==
  /\ phase = "idle"
  /\ phase' = "open"
  /\ fd' = fd
  /\ peer' = TRUE
  /\ lastErrno' = 0
  /\ lastOp' = "connect"
  /\ UNCHANGED fileVars

\* A second connect does not open another entry.
ConnectAgain ==
  /\ phase = "open"
  /\ lastErrno' = SecondConnect
  /\ lastOp' = "connectAgain"
  /\ UNCHANGED <<socketVars, fileVars>>

\* send before connect, and send before accept, both refuse.
SendEarly ==
  /\ phase \in {"idle", "listening"}
  /\ lastErrno' = SendBefore
  /\ lastOp' = "send"
  /\ UNCHANGED <<socketVars, fileVars>>

\* shutdown finishes the connection and leaves the entry closed.
Shutdown ==
  /\ phase = "open"
  /\ phase' = "closed"
  /\ fd' = fd
  /\ peer' = FALSE
  /\ lastErrno' = 0
  /\ lastOp' = "shutdown"
  /\ UNCHANGED fileVars

\* close aborts and returns the table to empty.
Close ==
  /\ phase \in {"idle", "listening", "open", "closed"}
  /\ phase' = "empty"
  /\ fd' = 0
  /\ peer' = FALSE
  /\ lastErrno' = 0
  /\ lastOp' = "close"
  /\ UNCHANGED fileVars

\* The first open stores descriptor 3. Descriptors 0, 1, and 2 stay clear.
OpenFile ==
  /\ ~fileOpen
  /\ ~fileFlight
  /\ fileOpen' = TRUE
  /\ lastErrno' = 0
  /\ lastOp' = "open"
  /\ UNCHANGED <<socketVars, fileFlight, stored0, stored1, stored2>>

\* exchange has set the in-flight id and has not taken the completion yet.
BeginFlight ==
  /\ ~fileFlight
  /\ fileFlight' = TRUE
  /\ lastErrno' = 0
  /\ lastOp' = "begin"
  /\ UNCHANGED <<socketVars, fileOpen, stored0, stored1, stored2>>

\* A second call in that window returns 16 and does not take a slot.
BusyCall ==
  /\ fileFlight
  /\ lastErrno' = Busy
  /\ lastOp' = "busy"
  /\ UNCHANGED <<socketVars, fileOpen, fileFlight, stored0, stored1, stored2>>

\* The completion arrived. A later call is not busy.
EndFlight ==
  /\ fileFlight
  /\ fileFlight' = FALSE
  /\ lastErrno' = 0
  /\ lastOp' = "end"
  /\ UNCHANGED <<socketVars, fileOpen, stored0, stored1, stored2>>

\* close frees the file slot.
CloseFile ==
  /\ fileOpen
  /\ ~fileFlight
  /\ fileOpen' = FALSE
  /\ lastErrno' = 0
  /\ lastOp' = "closeFile"
  /\ UNCHANGED <<socketVars, fileFlight, stored0, stored1, stored2>>

Next ==
  \/ Socket
  \/ ListenOk
  \/ ListenDefault
  \/ ListenBound
  \/ PeerArrives
  \/ Accept
  \/ Connect
  \/ ConnectAgain
  \/ SendEarly
  \/ Shutdown
  \/ Close
  \/ OpenFile
  \/ BeginFlight
  \/ BusyCall
  \/ EndFlight
  \/ CloseFile

Spec == Init /\ [][Next]_vars

TypeOK ==
  /\ phase \in {"empty", "idle", "listening", "open", "closed"}
  /\ fd \in {0, FirstFd}
  /\ peer \in BOOLEAN
  /\ fileOpen \in BOOLEAN
  /\ fileFlight \in BOOLEAN
  /\ lastErrno \in {
       0, SecondSocket, ListenUnsupported, ListenInUse,
       SecondConnect, SendBefore, Busy
     }
  /\ lastOp \in {
       "init", "socket", "listen", "listenDefault", "listenBound", "peer",
       "accept", "connect", "connectAgain", "send", "shutdown", "close",
       "open", "begin", "busy", "end", "closeFile"
     }

\* Empty holds no number. Every other phase holds the one socket, number 3.
\* Open is reached only with the peer connected.
OneSocket ==
  \/ /\ phase = "empty" /\ fd = 0 /\ ~peer
  \/ /\ phase = "idle" /\ fd = FirstFd /\ ~peer
  \/ /\ phase = "listening" /\ fd = FirstFd
  \/ /\ phase = "open" /\ fd = FirstFd /\ peer
  \/ /\ phase = "closed" /\ fd = FirstFd /\ ~peer

\* Those errnos, and success, are the only exits of this model.
OnlyExits ==
  lastErrno \in {
    0, SecondSocket, ListenUnsupported, ListenInUse,
    SecondConnect, SendBefore, Busy
  }

EmptyAfterClose ==
  lastOp # "close" \/ (phase = "empty" /\ fd = 0 /\ ~peer)

ClosedNotEmpty ==
  lastOp # "shutdown" \/ (phase = "closed" /\ fd = FirstFd)

ListenSameSocket ==
  lastOp # "listen" \/ (phase = "listening" /\ fd = FirstFd)

AcceptWaits ==
  lastOp # "accept" \/ (phase = "open" /\ peer /\ fd = FirstFd)

StdioFree ==
  ~StdioStored /\ ~stored0 /\ ~stored1 /\ ~stored2

FileBusy ==
  lastOp # "busy" \/ (fileFlight /\ lastErrno = Busy)

FileEmptyAfterClose ==
  lastOp # "closeFile" \/ ~fileOpen

=============================================================================
