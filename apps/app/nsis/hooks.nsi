!include "${__FILEDIR__}\gecl-ui.nsh"
!include "${__FILEDIR__}\gecl-cleanup.nsh"

!macro GECL_DATA_DIRECTORY NAME
  CreateDirectory "$INSTDIR\${NAME}"
  nsExec::ExecToStack '"$SYSDIR\icacls.exe" "$INSTDIR\${NAME}" /grant "*S-1-5-32-545:(OI)(CI)M"'
  Pop $0
  Pop $1
  ${If} $0 != 0
    MessageBox MB_ICONEXCLAMATION|MB_OK "无法设置 ${NAME} 的写入权限。首次启动时请选择其他可写的游戏目录。"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  !insertmacro GECL_DATA_DIRECTORY "profiles"
  !insertmacro GECL_DATA_DIRECTORY "meta"
  !insertmacro GECL_DATA_DIRECTORY "caches"
  !insertmacro GECL_DATA_DIRECTORY "store"
  !insertmacro GECL_DATA_DIRECTORY "icons"
  !insertmacro GECL_DATA_DIRECTORY "synced-options"
  ClearErrors
  FileOpen $0 "$INSTDIR\store.lock" a
  IfErrors gecl_lock_failed
  FileClose $0
  nsExec::ExecToStack '"$SYSDIR\icacls.exe" "$INSTDIR\store.lock" /grant "*S-1-5-32-545:M"'
  Pop $0
  Pop $1
  ${If} $0 != 0
    Goto gecl_lock_failed
  ${EndIf}
  Goto gecl_lock_done
  gecl_lock_failed:
    MessageBox MB_ICONEXCLAMATION|MB_OK "无法设置游戏目录锁的写入权限。首次启动时请选择其他可写目录。"
  gecl_lock_done:
!macroend
