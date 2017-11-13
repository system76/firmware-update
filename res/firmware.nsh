@echo -off

set BASEDIR \system76-firmware-update\firmware

if exist "fs0:%BASEDIR%" then
    fs0:
endif

if exist "fs1:%BASEDIR%" then
    fs1:
endif

if exist "fs2:%BASEDIR%" then
    fs2:
endif

if exist "fs3:%BASEDIR%" then
    fs3:
endif

if exist "fs4:%BASEDIR%" then
    fs4:
endif

if exist "fs5:%BASEDIR%" then
    fs5:
endif

if exist "fs6:%BASEDIR%" then
    fs6:
endif

if exist "fs7:%BASEDIR%" then
    fs7:
endif

if exist "fs8:%BASEDIR%" then
    fs8:
endif

if exist "fs9:%BASEDIR%" then
    fs9:
endif

if not exist "%BASEDIR%" then
    echo "Did not find %BASEDIR%"
    exit 1
endif

cd "%BASEDIR%"

if "%1" == "bios" then
    if "%2" == "flash" then
        if exist bios.nsh then
            bios.nsh
        else
            afuefi.efi bios.rom /B /N /P /Q
        endif
        exit %lasterror%
    endif

    if "%2" == "verify" then
        afuefi.efi bios.rom /D /Q
        exit %lasterror%
    endif

    echo "bios: unknown subcommand '%2'"
    exit 1
endif

if "%1" == "ec" then
    if "%2" == "flash" then
        uecflash.efi ec.rom /AD /F2 /P
        exit %lasterror%
    endif

    echo "ec: unknown subcommand '%2'"
    exit 1
endif

if "%1" == "ec2" then
    if "%2" == "flash" then
        uecflash.efi ec2.rom /AD /O2 /P
        exit %lasterror%
    endif

    echo "ec2: unknown subcommand '%2'"
    exit 1
endif

if "%1" == "me" then
    if "%2" == "flash" then
        if exist meset.tag then
            rm meset.tag
            fpt.efi -F "%BASEDIR%\me.rom" -P "%BASEDIR%\fparts.txt"
            exit %lasterror%
        else
            echo > meset.tag
            meset.efi
            stall 10000000
            exit 1
        endif
    endif

    echo "me: unknown subcommand '%2'"
    exit 1
endif

echo "unknown command '%1'"
exit 1
