!include LogicLib.nsh
!include FileFunc.nsh

!macro GECL_CLEANUP_FUNCTIONS PREFIX
Var GeclCleanupFailed
Function ${PREFIX}GeclRemoveDataTree
  Exch $0
  Push $1
  Push $2
  Push $3
  System::Call 'kernel32::GetFileAttributesW(w r0) i .r1'
  ${If} $1 == -1
    Goto gecl_tree_done
  ${EndIf}
  IntOp $2 $1 & 0x400
  ${If} $2 != 0
    IntOp $2 $1 & 0x10
    ${If} $2 != 0
      RMDir "$0"
    ${Else}
      Delete "$0"
    ${EndIf}
    Goto gecl_tree_done
  ${EndIf}
  IntOp $2 $1 & 0x10
  ${If} $2 == 0
    SetFileAttributes "$0" NORMAL
    Delete "$0"
    Goto gecl_tree_done
  ${EndIf}
  FindFirst $1 $2 "$0\*"
  ${DoWhile} $2 != ""
    ${If} $2 != "."
    ${AndIf} $2 != ".."
      Push "$0\$2"
      Call ${PREFIX}GeclRemoveDataTree
    ${EndIf}
    FindNext $1 $2
  ${Loop}
  FindClose $1
  RMDir "$0"
  gecl_tree_done:
  Pop $3
  Pop $2
  Pop $1
  Pop $0
FunctionEnd

Function ${PREFIX}GeclCleanupData
  ${If} $UpdateMode == 1
    Return
  ${EndIf}
  Push $0
  Push $1
  GetFullPathName $0 "$INSTDIR"
  ${GetParent} "$0" $1
  ${If} $1 == ""
    Goto gecl_cleanup_done
  ${EndIf}
  ${If} $0 == "$WINDIR"
  ${OrIf} $0 == "$PROGRAMFILES32"
  ${OrIf} $0 == "$PROGRAMFILES64"
  ${OrIf} $0 == "$PROFILE"
    Goto gecl_cleanup_done
  ${EndIf}
  System::Call 'kernel32::GetFileAttributesW(w r0) i .r1'
  IntOp $1 $1 & 0x400
  ${If} $1 != 0
    Goto gecl_cleanup_done
  ${EndIf}
  ${If} $DeleteAppDataCheckboxState == 1
    StrCpy $GeclCleanupFailed 0
    !insertmacro GECL_REMOVE_DATA "${PREFIX}" "profiles"
    !insertmacro GECL_REMOVE_DATA "${PREFIX}" "meta"
    !insertmacro GECL_REMOVE_DATA "${PREFIX}" "caches"
    !insertmacro GECL_REMOVE_DATA "${PREFIX}" "store"
    !insertmacro GECL_REMOVE_DATA "${PREFIX}" "icons"
    !insertmacro GECL_REMOVE_DATA "${PREFIX}" "synced-options"
    ${If} $GeclCleanupFailed == 1
      MessageBox MB_ICONEXCLAMATION|MB_OK "部分游戏数据无法删除，请关闭占用文件的程序后检查安装目录。"
    ${EndIf}
  ${Else}
    System::Call 'kernel32::GetFileAttributesW(w "$INSTDIR\store") i .r1'
    IntOp $1 $1 & 0x400
    ${If} $1 == 0
      RMDir "$INSTDIR\store\content"
      RMDir "$INSTDIR\store\staging"
    ${EndIf}
    RMDir "$INSTDIR\profiles"
    RMDir "$INSTDIR\meta"
    RMDir "$INSTDIR\caches"
    RMDir "$INSTDIR\store"
    RMDir "$INSTDIR\icons"
    RMDir "$INSTDIR\synced-options"
  ${EndIf}
  Delete "$INSTDIR\store.lock"
  RMDir "$INSTDIR"
  gecl_cleanup_done:
  Pop $1
  Pop $0
FunctionEnd
!macroend

!macro GECL_REMOVE_DATA PREFIX NAME
  Push "$INSTDIR\${NAME}"
  Call ${PREFIX}GeclRemoveDataTree
  IfFileExists "$INSTDIR\${NAME}\." 0 +2
    StrCpy $GeclCleanupFailed 1
!macroend
