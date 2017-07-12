@echo -off

set BASEDIR \system76-fu\firmware

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
        echo "Flashing BIOS"
        afuefi.efi bios.rom /B /P /N
        exit %lasterror%
    endif

    if "%2" == "verify" then
        echo "Verifying BIOS"
        afuefi.efi bios.rom /D
        exit %lasterror%
    endif

    echo "bios: unknown subcommand '%2'"
    exit 1
endif

if "%1" == "ec" then
    if "%2" == "flash" then
        echo "Flashing EC"
        uecflash.efi ec.rom /AD /F2 /P
        exit %lasterror%
    endif

    echo "bios: unknown subcommand '%2'"
    exit 1
endif

echo "unknown command '%1'"
exit 1
