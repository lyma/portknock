# Remove old readonly constants from session
Remove-Variable -Name KNOCK_DESTINATION -Force -ErrorAction SilentlyContinue
Remove-Variable -Name KNOCK_VALID_TYPES -Force -ErrorAction SilentlyContinue
Remove-Variable -Name KNOCK_PORTS -Force -ErrorAction SilentlyContinue
Remove-Variable -Name KNOCK_EXE_TARGET -Force -ErrorAction SilentlyContinue

# === SCRIPT CONFIGURATION ===
Set-Variable KNOCK_DESTINATION -Option ReadOnly -Value "1.2.3.4"
Set-Variable KNOCK_VALID_TYPES -Option ReadOnly -Value ("TCP", "UDP")
Set-Variable KNOCK_PORTS -Option ReadOnly -Value ((1, "TCP"), (2, "TCP"), (3, "UDP"), (4, "UDP"))
Set-Variable KNOCK_EXE_TARGET -Option ReadOnly -Value "mstsc /v:$KNOCK_DESTINATION /prompt"
# === END OF SCRIPT CONFIGURATION ===

# Knock all configured ports in the correct order
$KNOCK_PORTS | foreach {
    $knockPort = $_[0]
    $knockType = $_[1]

    # Make sure that no invalid knock type was specified    
    if ( -Not $KNOCK_VALID_TYPES.Contains($knockType) ) {
        Write-Error "Invalid knock type specified: $knockType"
        Exit(1)
    } else {
        Write-Host "Knocking $knockType port $knockPort..."

        # Execute the port knock, either TCP or UDP
        switch($knockType) {
            "TCP" {
                $tcpClient = New-Object System.Net.Sockets.TcpClient
                $tcpClient.BeginConnect($KNOCK_DESTINATION, $knockPort, $null, $null) | Out-Null
                $tcpClient.Close() | Out-Null
            }
            "UDP" {
                $udpClient = New-Object System.Net.Sockets.UdpClient
                $udpClient.Connect($KNOCK_DESTINATION, $knockPort) | Out-Null
                $udpClient.Send([byte[]](0), 1) | Out-Null
                $udpClient.Close() | Out-Null
            }
        }

        # Wait a second to make sure that our firewall gets the packets in the right order
        sleep 1 
    }
}

# Start the configured service
Write-Host "Open sesame!"
Write-Host "Executing target command...: $KNOCK_EXE_TARGET"
Invoke-Expression -Command $KNOCK_EXE_TARGET
