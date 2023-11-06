@echo -off

if exist "fs0:%1" then
    fs0:
endif

if exist "fs1:%1" then
    fs1:
endif

if exist "fs2:%1" then
    fs2:
endif

if exist "fs3:%1" then
    fs3:
endif

if exist "fs4:%1" then
    fs4:
endif

if exist "fs5:%1" then
    fs5:
endif

if exist "fs6:%1" then
    fs6:
endif

if exist "fs7:%1" then
    fs7:
endif

if exist "fs8:%1" then
    fs8:
endif

if exist "fs9:%1" then
    fs9:
endif

if not exist "%1" then
    echo "Did not find %1"
    exit 1
endif

cd "%1"

if "%2" == "bios" then
    if "%3" == "flash" then
        # Flash with firmware.nsh script and exit if possible
        if exist firmware.nsh then
            firmware.nsh
            exit %lasterror%
        endif

        echo "bios: no flash implementation found"
        exit 1
    endif

    echo "bios: unknown subcommand '%3'"
    exit 1
endif

if "%2" == "ec" then
    if "%3" == "flash" then
        # Flash with ec.nsh script and exit if possible
        if exist ec.nsh then
            ec.nsh
            exit %lasterror%
        endif

        echo "ec.nsh not found"
        exit 1
    endif

    echo "ec: unknown subcommand '%3'"
    exit 1
endif

echo "unknown command '%2'"
exit 1
