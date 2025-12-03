param(
    [int]$Port = 18080,
    [string]$DbCommand = ""
)

& "$PSScriptRoot/start_db.ps1" -Command $DbCommand
& "$PSScriptRoot/start_server.ps1" -Port $Port
