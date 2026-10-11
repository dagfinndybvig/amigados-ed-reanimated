$p = Start-Process msiexec -ArgumentList '/i','C:\Users\dybvig\AppData\Local\Temp\AF10.msi','/qn','/norestart','AMIGA_FILES=C:\AmigaFiles','/L*v','C:\Users\dybvig\AppData\Local\Temp\AF10-install2.log' -Wait -PassThru
Write-Output ('ExitCode: ' + $p.ExitCode)
