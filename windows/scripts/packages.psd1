@{
    SystemTools        = @{
        CMake    = @{
            WingetId = 'Kitware.CMake'
            Verify   = 'cmake'
        }
        Git      = @{
            WingetId = 'Git.Git'
            Verify   = 'git'
        }
        Ninja    = @{
            WingetId = 'Ninja-build.Ninja'
            Verify   = 'ninja'
        }
        SevenZip = @{
            WingetId = '7zip.7zip'
            Verify   = 'C:\Program Files\7-Zip\7z.exe'
        }
    }

    WingetPackages = @{
        LLVM = @{
            WingetId = 'LLVM.LLVM'
            Verify   = 'C:\Program Files\LLVM\bin\clang.exe'
        }
        FFmpeg = @{
            WingetId = 'Gyan.FFmpeg'
            Verify   = 'ffmpeg'
        }
        VulkanSDK = @{
            WingetId = 'KhronosGroup.VulkanSDK'
            Verify   = 'C:\VulkanSDK\*\Include\vulkan\vulkan.h'
        }
    }

    BinaryPackages = @{}

    HeaderOnlyPackages = @{}

    SourcePackages     = @{}

    VcpkgPackages = @{}

    SpecialPackages    = @{
        VisualStudio = @{
            Verify  = 'C:\Program Files (x86)\Microsoft Visual Studio\Installer\vswhere.exe'
            Message = 'Visual Studio with C++ workload required. Download from https://visualstudio.microsoft.com/'
        }
    }

    MayaFlux           = @{
        InstallRoot    = 'C:\MayaFlux'
        Subdirectories = @('include', 'lib', 'bin')
    }
}
