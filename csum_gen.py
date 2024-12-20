command = "PSTMRESTOREPAR"

res = 0 
for i in command:
    res = res^ ord(i)

print(bytes(command[0],encoding="utf8"))
print(res)