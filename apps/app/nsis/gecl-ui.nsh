!include nsDialogs.nsh
!addplugindir "${__FILEDIR__}\..\..\..\target\nsis-plugin"

!macro GECL_UI_FUNCTIONS
Var GeclPage
Var GeclDirectory
Var GeclBrowse
Var GeclHeadingFont
Var GeclBodyFont
Var GeclTitle
Var GeclLaunch
Var GeclShortcut
Var GeclIcon

Function GeclTheme
  CreateFont $GeclBodyFont "Microsoft YaHei UI" 9 400
  SetCtlColors $HWNDPARENT "202832" "FFFFFF"
  ShowWindow $mui.Header.Image ${SW_HIDE}
  GetDlgItem $0 $HWNDPARENT 1
  SendMessage $0 ${WM_SETFONT} $GeclBodyFont 1
  gecl_ui::Style "$0" "1"
  GetDlgItem $0 $HWNDPARENT 2
  SendMessage $0 ${WM_SETFONT} $GeclBodyFont 1
  gecl_ui::Style "$0" "0"
FunctionEnd

Function GeclPageFont
  System::Call 'user32::GetWindow(p $GeclPage, i 5) p .r0'
  ${DoWhile} $0 != 0
    SendMessage $0 ${WM_SETFONT} $GeclBodyFont 1
    System::Call 'user32::GetWindow(p r0, i 2) p .r0'
  ${Loop}
  SendMessage $GeclTitle ${WM_SETFONT} $GeclHeadingFont 1
FunctionEnd

Function GeclLogo
  InitPluginsDir
  File /oname=$PLUGINSDIR\gecl.ico "${INSTALLERICON}"
  ${NSD_CreateIcon} 0 4u 36u 36u ""
  Pop $0
  ${NSD_SetIcon} $0 "$PLUGINSDIR\gecl.ico" $GeclIcon
  ${NSD_CreateLabel} 48u 3u 200u 24u "gecl"
  Pop $GeclTitle
  CreateFont $GeclHeadingFont "Microsoft YaHei UI" 20 700
  SendMessage $GeclTitle ${WM_SETFONT} $GeclHeadingFont 1
  SetCtlColors $GeclTitle "202832" "FFFFFF"
  ${NSD_CreateLabel} 49u 29u 220u 14u "${VERSION}"
  Pop $0
  SetCtlColors $0 "596370" "FFFFFF"
FunctionEnd

Function GeclDirectoryPage
  Call SkipIfPassive
  !insertmacro MUI_HEADER_TEXT "安装 gecl" "选择安装位置"
  nsDialogs::Create 1018
  Pop $GeclPage
  SetCtlColors $GeclPage "202832" "FFFFFF"
  Call GeclTheme
  Call GeclLogo
  ${NSD_CreateLabel} 0 61u 100% 14u "安装位置"
  Pop $0
  SetCtlColors $0 "202832" "FFFFFF"
  ${NSD_CreateText} 0 80u -60u 13u "$INSTDIR"
  Pop $GeclDirectory
  SendMessage $GeclDirectory ${EM_SETREADONLY} 1 0
  SetCtlColors $GeclDirectory "202832" "F8F9FA"
  ${NSD_CreateButton} -53u 78u 53u 17u "更改…"
  Pop $GeclBrowse
  ${NSD_OnClick} $GeclBrowse GeclChooseDirectory
  GetDlgItem $0 $HWNDPARENT 1
  SendMessage $0 ${WM_SETTEXT} 0 "STR:安装"
  Call GeclPageFont
  gecl_ui::Style "$GeclBrowse" "0"
  nsDialogs::Show
FunctionEnd

Function GeclChooseDirectory
  nsDialogs::SelectFolderDialog "选择安装位置" "$INSTDIR"
  Pop $0
  ${If} $0 == "error"
    Return
  ${EndIf}
  StrCpy $INSTDIR $0
  Call GeclAppendDirectory
  ${NSD_SetText} $GeclDirectory "$INSTDIR"
FunctionEnd

Function GeclAppendDirectory
  StrCpy $1 "$INSTDIR" 1 -1
  ${If} $1 == "\"
    StrCpy $INSTDIR "$INSTDIR" -1
  ${EndIf}
  ${GetFileName} "$INSTDIR" $1
  ${If} $1 != "GeCL"
    StrCpy $INSTDIR "$INSTDIR\GeCL"
  ${EndIf}
FunctionEnd

Function GeclDirectoryLeave
  ${If} $INSTDIR == ""
    MessageBox MB_ICONEXCLAMATION|MB_OK "请选择安装位置。"
    Abort
  ${EndIf}
FunctionEnd

Function GeclProgressShow
  Call GeclTheme
  SetCtlColors $mui.InstFilesPage "202832" "FFFFFF"
  GetDlgItem $0 $mui.InstFilesPage 1004
  System::Call 'uxtheme::SetWindowTheme(p r0, w "", w "")'
  SendMessage $0 0x0409 0 0xCA7914
FunctionEnd

Function GeclFinishPage
  Call SkipIfPassive
  !insertmacro MUI_HEADER_TEXT "安装完成" "gecl 已安装到所选目录"
  nsDialogs::Create 1018
  Pop $GeclPage
  SetCtlColors $GeclPage "202832" "FFFFFF"
  Call GeclTheme
  Call GeclLogo
  ${NSD_CreateLabel} 0 61u 100% 24u "$INSTDIR"
  Pop $0
  SetCtlColors $0 "596370" "FFFFFF"
  ${NSD_CreateCheckbox} 0 98u 100% 16u "启动 gecl"
  Pop $GeclLaunch
  ${NSD_Check} $GeclLaunch
  SetCtlColors $GeclLaunch "202832" "FFFFFF"
  ${NSD_CreateCheckbox} 0 122u 100% 16u "创建桌面快捷方式"
  Pop $GeclShortcut
  ${NSD_Check} $GeclShortcut
  SetCtlColors $GeclShortcut "202832" "FFFFFF"
  GetDlgItem $0 $HWNDPARENT 1
  SendMessage $0 ${WM_SETTEXT} 0 "STR:完成"
  Call GeclPageFont
  nsDialogs::Show
FunctionEnd

Function GeclFinishLeave
  ${NSD_GetState} $GeclShortcut $0
  ${If} $0 == ${BST_CHECKED}
    Call CreateOrUpdateDesktopShortcut
  ${EndIf}
  ${NSD_GetState} $GeclLaunch $0
  ${If} $0 == ${BST_CHECKED}
    Call RunMainBinary
  ${EndIf}
FunctionEnd
!macroend
