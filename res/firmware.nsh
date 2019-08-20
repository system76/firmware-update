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
        if exist meset.efi then
            if exist meset.tag then
                rm meset.tag
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

        if exist fpt.efi then
            fpt.efi -P "%1\fparts.txt" -F "%1\firmware.rom"
            exit %lasterror%
        else
            if exist efiflash.efi then
                efiflash.efi firmware.rom /C /NR \NoOemId
                exit %lasterror%
            else
                if exist uefiflash.efi then
                    if exist uefiflash.tag then
                        rm uefiflash.tag
                    else
                        echo > uefiflash.tag
                        if not exist uefiflash.tag then
                            echo "failed to create uefiflash.tag"
                            exit 1
                        endif

                        uefiflash.efi firmware.rom
                        exit 1
                    endif
                endif
            endif
        endif
    endif

    echo "bios: unknown subcommand '%3'"
    exit 1
endif

if "%2" == "ec" then
    if "%3" == "flash" then
        uecflash.efi ec.rom /AD /F2 /P
        exit %lasterror%
    endif

    echo "ec: unknown subcommand '%3'"
    exit 1
endif

if "%2" == "ec2" then
    if "%3" == "flash" then
        uecflash.efi ec2.rom /AD /O2 /P
        exit %lasterror%
    endif

    echo "ec2: unknown subcommand '%3'"
    exit 1
endif

echo "unknown command '%2'"
exit 1
