@echo off

rmdir /s /q target\debug\resources 2>nul
rmdir /s /q target\release\resources 2>nul
rmdir /s /q target\rwd\resources 2>nul

mkdir target\debug\resources 2>nul
mkdir target\debug\debug_out 2>nul
mkdir target\debug\config 2>nul

mkdir target\release\resources 2>nul
mkdir target\release\debug_out 2>nul
mkdir target\release\config 2>nul

mkdir target\rwd\resources 2>nul
mkdir target\rwd\debug_out 2>nul
mkdir target\rwd\config 2>nul

xcopy /e /i /y resources target\debug\resources
xcopy /e /i /y libs target\debug
xcopy /e /i /y debug_out target\debug\debug_out
xcopy /e /i /y config target\debug\config

xcopy /e /i /y resources target\release\resources
xcopy /e /i /y libs target\release
xcopy /e /i /y debug_out target\release\debug_out
xcopy /e /i /y config target\release\config

xcopy /e /i /y resources target\rwd\resources
xcopy /e /i /y libs target\rwd
xcopy /e /i /y debug_out target\rwd\debug_out
xcopy /e /i /y config target\rwd\config

REM Cleanup some files:
rmdir /s /q debug_out 2>nul
mkdir debug_out

