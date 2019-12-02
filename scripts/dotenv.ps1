Set-StrictMode -Version Latest
if (Test-Path ".env")
{
    $content = Get-Content ".env" -ErrorAction Stop
    foreach ($line in $content)
    {
        $kvp = $line -split "=",2
        [Environment]::SetEnvironmentVariable($kvp[0], $kvp[1], "Process") | Out-Null
    }
}

& $args[0] $args[1..-1]
