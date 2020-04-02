#Region ;**** Directives created by AutoIt3Wrapper_GUI ****
#AutoIt3Wrapper_outfile=portknock.exe
#EndRegion ;**** Directives created by AutoIt3Wrapper_GUI ****
#include <ButtonConstants.au3>
#include <ComboConstants.au3>
#include <EditConstants.au3>
#include <GUIConstantsEx.au3>
#include <GUIListBox.au3>
#include <StaticConstants.au3>
#include <WindowsConstants.au3>
#include <file.au3>
#include <Array.au3>
FileInstall("C:\Documents and Settings\greg\Desktop\autoit\portknock\TCP.exe", @ScriptDir & "\TCP.exe",0)
;knock it like it's hot
$g_IP = ""
$g_port = ""
$ListLocation = 1
 
;check config file
if not FileExists(@ScriptDir & "\config.txt") Then
	;create it with dummy info
	$file = FileOpen(@ScriptDir & "\config.txt", 1)
	If $file = -1 Then
		MsgBox(0, "Error", "Unable to open file.")
		Exit
	EndIf
 
	FileWriteLine($file, "Description;IPAddress;UDP;Port;GregRocks")
 
	FileClose($file)
 
EndIf
 
ReadConfig ()
 
; Start The  Services
;==============================================
TCPStartUp()
UDPStartup()
 
;###########################################
 
#Region ### START Koda GUI section ### Form=C:\Documents and Settings\greg\Desktop\autoit\portknock\Form1.kxf
$Form1_1 = GUICreate("GregSowell.com Port Knock", 441, 434, 192, 114)
$List1 = GUICtrlCreateList("", 24, 16, 393, 201)
$CBO1 = GUICtrlCreateCombo("None", 33, 280, 65, 25)
GUICtrlSetData(-1, "TCP|UDP")
$In1Port = GUICtrlCreateInput("", 121, 280, 105, 21)
$In1Text = GUICtrlCreateInput("", 233, 280, 177, 21)
$Label1 = GUICtrlCreateLabel("1", 9, 280, 10, 17)
$BTNKnock = GUICtrlCreateButton("Knock", 96, 400, 73, 25, $WS_GROUP)
$BTNAdd = GUICtrlCreateButton("Add/Update", 182, 400, 73, 25, $WS_GROUP)
$BTNDelete = GUICtrlCreateButton("Delete", 272, 400, 73, 25, $WS_GROUP)
$InIP = GUICtrlCreateInput("", 40, 234, 161, 21)
$Label2 = GUICtrlCreateLabel("IP", 8, 234, 14, 17)
$Label3 = GUICtrlCreateLabel("Type", 44, 259, 28, 17)
$Label4 = GUICtrlCreateLabel("Port", 124, 259, 23, 17)
$Label5 = GUICtrlCreateLabel("Text", 238, 259, 25, 17)
$CBO2 = GUICtrlCreateCombo("None", 33, 309, 65, 25)
GUICtrlSetData(-1, "TCP|UDP")
$In2Port = GUICtrlCreateInput("", 121, 309, 105, 21)
$In2Text = GUICtrlCreateInput("", 233, 309, 177, 21)
$Label6 = GUICtrlCreateLabel("2", 9, 309, 10, 17)
$CBO3 = GUICtrlCreateCombo("None", 33, 341, 65, 25)
GUICtrlSetData(-1, "TCP|UDP")
$In3Port = GUICtrlCreateInput("", 121, 341, 105, 21)
$In3Text = GUICtrlCreateInput("", 233, 341, 177, 21)
$Label7 = GUICtrlCreateLabel("3", 9, 341, 10, 17)
$CBO4 = GUICtrlCreateCombo("None", 33, 367, 65, 25)
GUICtrlSetData(-1, "TCP|UDP")
$In4Port = GUICtrlCreateInput("", 121, 367, 105, 21)
$In4Text = GUICtrlCreateInput("", 233, 367, 177, 21)
$Label8 = GUICtrlCreateLabel("4", 9, 367, 10, 17)
$InDesc = GUICtrlCreateInput("", 253, 234, 161, 21)
$Label9 = GUICtrlCreateLabel("Desc", 221, 234, 29, 17)
GUISetState(@SW_SHOW)
#EndRegion ### END Koda GUI section ###
 
PopList()
 
 
While 1
	$nMsg = GUIGetMsg()
	Switch $nMsg
		Case $BTNAdd
			;
			$foundOne = 0
			for $y = 1 to $aConfig
				;loop through until we find the correct line
				if StringLeft($aConfig[$y], StringInStr($aConfig[$y], ";") - 1) == GUICtrlRead($InDesc) Then
					; we have our match, update
					$foundOne = $y
				EndIf
			Next
			if $foundOne == 0 Then
				;we didn't find a match above, so write it to file, then reload listbox
				$tempNewEntry = GUICtrlRead($InDesc) & ";" & GUICtrlRead($InIP) & ";" & GUICtrlRead($CBO1) & ";" & GUICtrlRead($In1Port) & ";" & GUICtrlRead($In1Text)
				if GUICtrlRead($CBO2) <> "None" Then
					;add 2
					$tempNewEntry = $tempNewEntry & ";" & GUICtrlRead($CBO2) & ";" & GUICtrlRead($In2Port) & ";" & GUICtrlRead($In2Text)
					if GUICtrlRead($CBO3) <> "None" Then
						;add 3
						$tempNewEntry = $tempNewEntry & ";" & GUICtrlRead($CBO3) & ";" & GUICtrlRead($In3Port) & ";" & GUICtrlRead($In3Text)
						if GUICtrlRead($CBO4) <> "None" Then
							;add 4
							$tempNewEntry = $tempNewEntry & ";" & GUICtrlRead($CBO4) & ";" & GUICtrlRead($In4Port) & ";" & GUICtrlRead($In4Text)
						EndIf
					EndIf
				EndIf
				_ArrayAdd($aConfig, $tempNewEntry)
			Else
				;does exist, and write over line $y
				$aConfig[$foundOne] = GUICtrlRead($InDesc) & ";" & GUICtrlRead($InIP) & ";" & GUICtrlRead($CBO1) & ";" & GUICtrlRead($In1Port) & ";" & GUICtrlRead($In1Text)
				if GUICtrlRead($CBO2) <> "None" Then
					;add 2
					$aConfig[$foundOne] = $aConfig[$foundOne] & ";" & GUICtrlRead($CBO2) & ";" & GUICtrlRead($In2Port) & ";" & GUICtrlRead($In2Text)
					if GUICtrlRead($CBO3) <> "None" Then
						;add 3
						$aConfig[$foundOne] = $aConfig[$foundOne] & ";" & GUICtrlRead($CBO3) & ";" & GUICtrlRead($In3Port) & ";" & GUICtrlRead($In3Text)
						if GUICtrlRead($CBO4) <> "None" Then
							;add 4
							$aConfig[$foundOne] = $aConfig[$foundOne] & ";" & GUICtrlRead($CBO4) & ";" & GUICtrlRead($In4Port) & ";" & GUICtrlRead($In4Text)
						EndIf
					EndIf
				EndIf
			EndIf
			;write array to file
			_FileWriteFromArray(@ScriptDir & "\config.txt", $aConfig,1)
			ReadConfig()
			PopList()
 
		Case $BTNDelete
			;
			for $y = 1 to $aConfig[0]
				;loop through until we find the correct line
				if StringLeft($aConfig[$y], StringInStr($aConfig[$y], ";") - 1) == GUICtrlRead($InDesc) Then
					; we have our match, update
					_ArrayDelete($aConfig, $y)
					_FileWriteFromArray(@ScriptDir & "\config.txt", $aConfig,1)
					ReadConfig()
					PopList()
				EndIf
			Next
 
		Case $BTNKnock
			;
			$g_IP = GUICtrlRead($InIP)
			$g_port = GUICtrlRead($In1Port)
			$g_text = ""
			$g_text = GUICtrlRead($In1Text)
			if GUICtrlRead($CBO1) == "TCP" Then
				TCPKnock()
			Else
				UDPKnock()
			EndIf
			sleep(300)
			if GUICtrlRead($CBO2) <> "None" Then
				$g_port = GUICtrlRead($In2Port)
				$g_text = ""
				$g_text = GUICtrlRead($In2Text)
			EndIf
			if GUICtrlRead($CBO2) == "TCP" Then
				TCPKnock()
			Elseif GUICtrlRead($CBO2) == "UDP" Then
				UDPKnock()
			EndIf
			sleep(300)
			if GUICtrlRead($CBO3) <> "None" Then
				$g_port = GUICtrlRead($In3Port)
				$g_text = ""
				$g_text = GUICtrlRead($In3Text)
			EndIf
			if GUICtrlRead($CBO3) == "TCP" Then
				TCPKnock()
			Elseif GUICtrlRead($CBO3) == "UDP" Then
				UDPKnock()
			EndIf
			sleep(300)
			if GUICtrlRead($CBO4) <> "None" Then
				$g_port = GUICtrlRead($In4Port)
				$g_text = ""
				$g_text = GUICtrlRead($In4Text)
			EndIf
			if GUICtrlRead($CBO4) == "TCP" Then
				TCPKnock()
			Elseif GUICtrlRead($CBO4) == "UDP" Then
				UDPKnock()
			EndIf
			ToolTip("knock complete")
			sleep(5000)
			ToolTip("")
 
		case $GUI_EVENT_PRIMARYUP
			;mouse was pressed, lets check to see if they choose a new item in list
			;check which list item is highlighted
			$tempList = GUICtrlRead($List1)
			;see if this is new item chosen or just a click somewhere on the prog
			if $tempList <> $ListLocation and $tempList <> "" Then
				;change, update everything
				;set list location to the temp value
				$ListLocation = $tempList
 
				;clear the entries
				GUICtrlSetData($InDesc,"")
				GUICtrlSetData($InIP,"")
				GUICtrlSetData($CBO1,"None")
				GUICtrlSetData($In1Port,"")
				GUICtrlSetData($In1Text,"")
				GUICtrlSetData($CBO2,"None")
				GUICtrlSetData($In2Port,"")
				GUICtrlSetData($In2Text,"")
				GUICtrlSetData($CBO3,"None")
				GUICtrlSetData($In3Port,"")
				GUICtrlSetData($In3Text,"")
				GUICtrlSetData($CBO4,"None")
				GUICtrlSetData($In4Port,"")
				GUICtrlSetData($In4Text,"")
				;set the entries
				for $y = 1 to $aConfig[0]
					;loop through until we find the correct line
					if StringLeft($aConfig[$y], StringInStr($aConfig[$y], ";") - 1) == $ListLocation Then
						; we have our match
						$ConfigLine = $aConfig[$y]
					EndIf
				Next
				;fill in all the boxes
				GUICtrlSetData($InDesc, $ListLocation)
				GUICtrlSetData($InIP, StringMid($ConfigLine,StringInStr($ConfigLine,";") + 1, StringInStr($ConfigLine,";",0,2) - StringInStr($ConfigLine,";") - 1))
				GUICtrlSetData($CBO1, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,2) + 1, StringInStr($ConfigLine,";",0,3) - StringInStr($ConfigLine,";",0,2) - 1))
				GUICtrlSetData($In1Port, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,3) + 1, StringInStr($ConfigLine,";",0,4) - StringInStr($ConfigLine,";",0,3) - 1))
				GUICtrlSetData($In1Text, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,4) + 1, StringInStr($ConfigLine,";",0,5) - StringInStr($ConfigLine,";",0,4) - 1))
				$tempstring = StringReplace($ConfigLine, ";", ";")
				$tempCount = @extended
				if $tempCount > 4 Then
					;we have a second set
					GUICtrlSetData($CBO2,  StringMid($ConfigLine,StringInStr($ConfigLine,";",0,5) + 1, StringInStr($ConfigLine,";",0,6) - StringInStr($ConfigLine,";",0,5) - 1))
					GUICtrlSetData($In2Port, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,6) + 1, StringInStr($ConfigLine,";",0,7) - StringInStr($ConfigLine,";",0,6) - 1))
					GUICtrlSetData($In2Text, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,7) + 1, StringInStr($ConfigLine,";",0,8) - StringInStr($ConfigLine,";",0,7) - 1))
				EndIf
				if $tempCount > 7 Then
					;we have a third set
					GUICtrlSetData($CBO3, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,8) + 1, StringInStr($ConfigLine,";",0,9) - StringInStr($ConfigLine,";",0,8) - 1))
					GUICtrlSetData($In3Port, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,9) + 1, StringInStr($ConfigLine,";",0,10) - StringInStr($ConfigLine,";",0,9) - 1))
					GUICtrlSetData($In3Text, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,10) + 1, StringInStr($ConfigLine,";",0,11) - StringInStr($ConfigLine,";",0,10) - 1))
				EndIf
				if $tempCount > 10 Then
					;we have a fourth set
					GUICtrlSetData($CBO4, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,11) + 1, StringInStr($ConfigLine,";",0,12) - StringInStr($ConfigLine,";",0,11) - 1))
					GUICtrlSetData($In4Port, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,12) + 1, StringInStr($ConfigLine,";",0,13) - StringInStr($ConfigLine,";",0,12) - 1))
					GUICtrlSetData($In4Text, StringMid($ConfigLine,StringInStr($ConfigLine,";",0,4) + 1))
				EndIf
 
			EndIf
 
		Case $GUI_EVENT_CLOSE
			TCPShutdown()
			UDPShutdown()
			Exit
 
	EndSwitch
WEnd
;###########################################
 
Func PopList ()
	;populate list box
	GUICtrlSetData($List1, "")
	for $x = 1 to $aConfig[0]
		GUICtrlSetData($List1, StringLeft($aConfig[$x], StringInStr($aConfig[$x], ";") - 1))
	Next
EndFunc
 
Func ReadConfig ()
	Global $aConfig
	If Not _FileReadToArray(@ScriptDir & "\config.txt",$aConfig) Then
		MsgBox(4096,"Error", " Error reading log to Array     error:" & @error)
		Exit
	EndIf
EndFunc
 
;knock functions
Func TCPKnock ()
	ToolTip("knocking " & $g_IP & " " & $g_port)
	run("""" & @ScriptDir & "\TCP.exe"" " & $g_IP & " " & $g_port)
	ToolTip("")
EndFunc
 
Func UDPKnock ()
	ToolTip("knocking " & $g_IP & " " & $g_port)
	$socket = UDPOpen($g_IP, $g_port)
	$status = UDPSend($socket, $g_text)
	UDPCloseSocket($socket)
	ToolTip("")
EndFunc
