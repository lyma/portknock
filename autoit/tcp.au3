#NoTrayIcon
;include <GUIConstantsEx.au3>

Opt('MustDeclareVars', 1)
; Set Some reusable info
   ;--------------------------
Local $ConnectedSocket, $szData
; Set $szIPADDRESS to wherever the SERVER is. We will change a PC name into an IP Address
;   Local $szServerPC = @ComputerName
;   Local $szIPADDRESS = TCPNameToIP($szServerPC)
Local $szIPADDRESS = $CmdLine[1]
Local $nPORT = $CmdLine[2]

; Start The TCP Services
;==============================================
TCPStartup()

; Initialize a variable to represent a connection
;==============================================
$ConnectedSocket = -1

;Attempt to connect to SERVER at its IP and PORT 33891
;=======================================================
$ConnectedSocket = TCPConnect($szIPADDRESS, $nPORT)
TCPCloseSocket($ConnectedSocket)