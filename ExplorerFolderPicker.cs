using System;
using System.IO;
using System.Runtime.InteropServices;

// Windows Common Item Dialog, configured to select folders (Explorer UI).
static class ExplorerFolderPicker {
    const uint Options = 0x20 | 0x40 | 0x800 | 0x8;
    [ComImport, Guid("42f85136-db7e-439c-85f1-e4075d135fc8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IFileDialog {
        [PreserveSig] int Show(IntPtr owner);
        void SetFileTypes(uint count, IntPtr filters);
        void SetFileTypeIndex(uint index);
        void GetFileTypeIndex(out uint index);
        void Advise(IntPtr events, out uint cookie);
        void Unadvise(uint cookie);
        void SetOptions(uint options);
        void GetOptions(out uint options);
        void SetDefaultFolder(IShellItem folder);
        void SetFolder(IShellItem folder);
        void GetFolder(out IShellItem folder);
        void GetCurrentSelection(out IShellItem item);
        void SetFileName([MarshalAs(UnmanagedType.LPWStr)] string name);
        void GetFileName(out IntPtr name);
        void SetTitle([MarshalAs(UnmanagedType.LPWStr)] string title);
        void SetOkButtonLabel([MarshalAs(UnmanagedType.LPWStr)] string label);
        void SetFileNameLabel([MarshalAs(UnmanagedType.LPWStr)] string label);
        void GetResult(out IShellItem result);
        void AddPlace(IShellItem folder, uint alignment);
        void SetDefaultExtension([MarshalAs(UnmanagedType.LPWStr)] string extension);
        void Close(int result);
        void SetClientGuid(ref Guid guid);
        void ClearClientData();
        void SetFilter(IntPtr filter);
    }
    [ComImport, Guid("43826d1e-e718-42ee-bc55-a1e261c37bfe"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IShellItem {
        void BindToHandler(IntPtr context, ref Guid handler, ref Guid iid, out IntPtr result);
        void GetParent(out IShellItem parent);
        void GetDisplayName(uint type, out IntPtr name);
        void GetAttributes(uint mask, out uint attributes);
        void Compare(IShellItem other, uint hints, out int order);
    }
    [DllImport("shell32.dll", CharSet=CharSet.Unicode, PreserveSig=false)]
    static extern void SHCreateItemFromParsingName(string name, IntPtr context, ref Guid iid, out IShellItem item);
    static IFileDialog Create(string initial) {
        var dialog=(IFileDialog)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("DC1C5A9C-E88A-4DDE-A5A1-60F82A20AEF7")));
        try {
            uint flags; dialog.GetOptions(out flags); dialog.SetOptions(flags | Options);
            dialog.SetTitle("Selecionar pasta da extensão CrOptix");
            dialog.SetOkButtonLabel("Selecionar pasta");
            if(Directory.Exists(initial)) {
                IShellItem folder; Guid iid=typeof(IShellItem).GUID;
                SHCreateItemFromParsingName(Path.GetFullPath(initial),IntPtr.Zero,ref iid,out folder);
                try { dialog.SetFolder(folder); } finally { Marshal.FinalReleaseComObject(folder); }
            }
            return dialog;
        } catch { Marshal.FinalReleaseComObject(dialog);throw; }
    }
    static string FilePath(IShellItem item) {
        IntPtr name; item.GetDisplayName(0x80058000,out name); // SIGDN_FILESYSPATH
        try { return Marshal.PtrToStringUni(name); } finally { Marshal.FreeCoTaskMem(name); }
    }
    public static string Select(IntPtr owner,string initial) {
        var dialog=Create(initial);
        try {
            int result=dialog.Show(owner);
            if(result==unchecked((int)0x800704C7))return null;
            Marshal.ThrowExceptionForHR(result);
            IShellItem item; dialog.GetResult(out item);
            try { return FilePath(item); } finally { Marshal.FinalReleaseComObject(item); }
        } finally { Marshal.FinalReleaseComObject(dialog); }
    }
    public static bool SelfTest(string folder) {
        var dialog=Create(folder);
        try {
            uint flags;dialog.GetOptions(out flags);
            IShellItem item;dialog.GetFolder(out item);
            try { return (flags&Options)==Options && string.Equals(FilePath(item),Path.GetFullPath(folder),StringComparison.OrdinalIgnoreCase); }
            finally { Marshal.FinalReleaseComObject(item); }
        } finally { Marshal.FinalReleaseComObject(dialog); }
    }
}
