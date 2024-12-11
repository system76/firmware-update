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

        # Unlock ME region if possible, should reboot automatically
        if exist meset.efi then
            # meset reboots automatically
            if exist meset.tag then
                rm meset.tag
                if exist meset.tag then
                    echo "failed to remove meset.tag"
                    exit 1
                endif
            else
                echo > meset.tag
                if not exist meset.tag then
                    echo "failed to create meset.tag"
                    exit 1
                endif

                meset.efi
                stall 10000000
                exit 1
            endif
        endif

        # Flash with FPT and exit if possible
        if exist fpt.efi then
            if exist fparts.txt then
                fpt.efi -P "%1\fparts.txt" -F "%1\firmware.rom" -DESC -Y
                if %lasterror% ne 0 then
                    exit %lasterror%
                endif
                fpt.efi -P "%1\fparts.txt" -F "%1\firmware.rom" -ME -Y
                if %lasterror% ne 0 then
                    exit %lasterror%
                endif
                fpt.efi -P "%1\fparts.txt" -F "%1\firmware.rom" -BIOS -Y
                exit %lasterror%
            else
                fpt.efi -F "%1\firmware.rom" -DESC -Y
                if %lasterror% ne 0 then
                    exit %lasterror%
                endif
                fpt.efi -F "%1\firmware.rom" -ME -Y
                if %lasterror% ne 0 then
                    exit %lasterror%
                endif
                fpt.efi -F "%1\firmware.rom" -BIOS -Y
                exit %lasterror%
            endif
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

        # Flash with uecflash and exit if possible
        if exist uecflash.efi then
            uecflash.efi ec.rom /AD /F2 /P /Y
            exit %lasterror%
        endif

        echo "ec: no flash implementation found"
        exit 1
    endif

    echo "ec: unknown subcommand '%3'"
    exit 1
endif

if "%2" == "ec2" then
    if "%3" == "flash" then
        # Flash with uecflash and exit
        if exist uecflash.efi then
            uecflash.efi ec2.rom /AD /O2 /P
            exit %lasterror%
        endif

        echo "ec2: no flash implementation found"
        exit 1
    endif

    echo "ec2: unknown subcommand '%3'"
    exit 1
endif

echo "unknown command '%2'"
exit 1
