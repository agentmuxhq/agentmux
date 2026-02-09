Get-Process | Where-Object {$_.Name -like '*wavemux*'} | Format-Table Name, Id, WorkingSet64 -AutoSize
