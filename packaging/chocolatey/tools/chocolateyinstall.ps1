$ErrorActionPreference = 'Stop'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$packageArgs = @{
    packageName   = 'floppy-warriors'
    fileType      = 'exe'
    url           = 'https://github.com/mlm-games/floppy-warriors/releases/latest'
    softwareName  = 'floppy-warriors'
    checksum      = ''
    checksumType  = 'sha256'
}
Install-ChocolateyPackage @packageArgs
