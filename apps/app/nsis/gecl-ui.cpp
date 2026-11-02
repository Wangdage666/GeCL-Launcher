#define UNICODE
#define _UNICODE
#include <windows.h>
#include <commctrl.h>
#include <cstdlib>
#include <cwchar>

struct StackEntry { StackEntry* next; wchar_t text[1]; };
static constexpr wchar_t roleKey[] = L"GeclInstallerButton";
static constexpr wchar_t hoverKey[] = L"GeclInstallerHover";
struct ExtraParameters {
    void* flags;
    int (WINAPI* execute)(int, HWND);
    void (WINAPI* validate)(wchar_t*);
    int (WINAPI* registerCallback)(HMODULE, UINT_PTR (__cdecl*)(int));
};
static HWND styled[128]{};
static unsigned styledCount=0;

static LRESULT CALLBACK ButtonProc(HWND button, UINT message, WPARAM w, LPARAM l, UINT_PTR id, DWORD_PTR) {
    if (message == WM_MOUSEMOVE) {
        if (!GetPropW(button, hoverKey)) {
            SetPropW(button, hoverKey, reinterpret_cast<HANDLE>(1));
            TRACKMOUSEEVENT track{sizeof(track), TME_LEAVE, button, 0};
            TrackMouseEvent(&track);
            InvalidateRect(button, nullptr, FALSE);
        }
    } else if (message == WM_MOUSELEAVE || message == WM_KILLFOCUS) {
        RemovePropW(button, hoverKey);
        InvalidateRect(button, nullptr, FALSE);
    } else if (message == WM_NCDESTROY) {
        RemovePropW(button, hoverKey);
        RemovePropW(button, roleKey);
        RemoveWindowSubclass(button, ButtonProc, id);
    }
    return DefSubclassProc(button, message, w, l);
}

static LRESULT CALLBACK ParentProc(HWND parent, UINT message, WPARAM w, LPARAM l, UINT_PTR id, DWORD_PTR) {
    if (message == WM_DRAWITEM) {
        auto* item = reinterpret_cast<DRAWITEMSTRUCT*>(l);
        if (item && item->CtlType == ODT_BUTTON && GetPropW(item->hwndItem, roleKey)) {
            const int savedDc = SaveDC(item->hDC);
            const bool primary = reinterpret_cast<UINT_PTR>(GetPropW(item->hwndItem, roleKey)) == 2;
            const bool disabled = (item->itemState & ODS_DISABLED) != 0;
            const bool pressed = (item->itemState & ODS_SELECTED) != 0;
            const bool hover = GetPropW(item->hwndItem, hoverKey) != nullptr;
            COLORREF background = primary ? RGB(20,121,202) : RGB(255,255,255);
            COLORREF foreground = primary ? RGB(255,255,255) : RGB(32,40,50);
            COLORREF border = primary ? background : RGB(207,214,221);
            if (disabled) { background=RGB(235,238,242); foreground=RGB(135,145,156); border=background; }
            else if (pressed) { background=primary?RGB(15,94,157):RGB(223,238,249); border=RGB(20,121,202); }
            else if (hover) { background=primary?RGB(17,107,179):RGB(237,244,250); border=RGB(20,121,202); }
            HBRUSH surface=CreateSolidBrush(RGB(255,255,255));
            FillRect(item->hDC,&item->rcItem,surface); DeleteObject(surface);
            HBRUSH brush=CreateSolidBrush(background);
            HPEN pen=CreatePen(PS_SOLID,1,border);
            auto oldBrush=SelectObject(item->hDC,brush);
            auto oldPen=SelectObject(item->hDC,pen);
            const int radius=MulDiv(7,GetDeviceCaps(item->hDC,LOGPIXELSX),96);
            RoundRect(item->hDC,item->rcItem.left,item->rcItem.top,item->rcItem.right,item->rcItem.bottom,radius,radius);
            SelectObject(item->hDC,oldPen); SelectObject(item->hDC,oldBrush);
            DeleteObject(pen); DeleteObject(brush);
            wchar_t text[256]{};
            GetWindowTextW(item->hwndItem,text,256);
            auto oldFont=SelectObject(item->hDC,reinterpret_cast<HFONT>(SendMessageW(item->hwndItem,WM_GETFONT,0,0)));
            SetBkMode(item->hDC,TRANSPARENT); SetTextColor(item->hDC,foreground);
            RECT label=item->rcItem;
            DrawTextW(item->hDC,text,-1,&label,DT_CENTER|DT_VCENTER|DT_SINGLELINE|DT_HIDEPREFIX);
            if ((item->itemState & ODS_FOCUS) && !(item->itemState & ODS_NOFOCUSRECT)) {
                InflateRect(&label,-4,-4); DrawFocusRect(item->hDC,&label);
            }
            SelectObject(item->hDC,oldFont);
            if (savedDc) RestoreDC(item->hDC,savedDc);
            return TRUE;
        }
    } else if (message == WM_NCDESTROY) RemoveWindowSubclass(parent,ParentProc,id);
    return DefSubclassProc(parent,message,w,l);
}

static UINT_PTR PopNumber(StackEntry** stack) {
    if (!stack || !*stack) return 0;
    auto* entry=*stack;
    UINT_PTR value=static_cast<UINT_PTR>(std::wcstoull(entry->text,nullptr,10));
    *stack=entry->next; GlobalFree(entry); return value;
}

static UINT_PTR __cdecl PluginEvent(int) {
    for (unsigned i=0;i<styledCount;++i) {
        if (IsWindow(styled[i])) {
            RemoveWindowSubclass(GetParent(styled[i]),ParentProc,1);
            RemoveWindowSubclass(styled[i],ButtonProc,1);
            RemovePropW(styled[i],roleKey);
            RemovePropW(styled[i],hoverKey);
        }
    }
    styledCount=0;
    return 0;
}

extern "C" void __cdecl Style(HWND, int, wchar_t*, StackEntry** stack, ExtraParameters* extra) {
    HWND button=reinterpret_cast<HWND>(PopNumber(stack));
    const auto role=PopNumber(stack);
    DWORD owner=0; GetWindowThreadProcessId(button,&owner);
    if (!IsWindow(button) || owner != GetCurrentProcessId()) return;
    HMODULE module=nullptr;
    GetModuleHandleExW(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS|GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,reinterpret_cast<LPCWSTR>(&PluginEvent),&module);
    if (!extra || extra->registerCallback(module,PluginEvent)<0) return;
    auto style=GetWindowLongPtrW(button,GWL_STYLE);
    SetPropW(button,roleKey,reinterpret_cast<HANDLE>(role?2:1));
    SetWindowLongPtrW(button,GWL_STYLE,(style & ~BS_TYPEMASK)|BS_OWNERDRAW);
    SetWindowSubclass(GetParent(button),ParentProc,1,0);
    SetWindowSubclass(button,ButtonProc,1,0);
    bool tracked=false;
    for (unsigned i=0;i<styledCount;++i) if (styled[i]==button) tracked=true;
    if (!tracked && styledCount<128) styled[styledCount++]=button;
    InvalidateRect(button,nullptr,TRUE);
}
