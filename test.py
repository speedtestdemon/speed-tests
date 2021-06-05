import pycurl
import sys
 
if len(sys.argv) != 2:
    raise ValueError('Please provide a url')
url = sys.argv[1]

def curl(url):
    c = pycurl.Curl()
    c.setopt(c.URL, url)
    c.setopt(c.NOBODY, 1)
    c.perform()
    return c

def printresults(results):
    print('time_namelookup: {0}'.format(results[0]))
    print('time_connect: {0}'.format(results[1]))
    print('time_appconnect: {0}'.format(results[2]))
    print('time_pretransfer: {0}'.format(results[3]))
    print('time_redirect: {0}'.format(results[4]))
    print('time_starttransfer: {0}'.format(results[5]))
    print('time to download: {0}'.format(results[6]))
    print('time_total: {0}'.format(results[7]))

# Turn data from cumulative seconds to individual seconds
def fixdata(D):
    i = len(D) - 3
    cumulativetime = D[ i+1 ]
    cur_i = i+1 # we start setting values at 2nd to last num.
    while i >= 0:
        if D[i] > 0:
            D[cur_i] = cumulativetime - D[i] # calculates new value for cur_i
            cumulativetime = D[i] # updates the new cumulative time.
            cur_i = i
        i -= 1
    return D

print('-------------------------------------------------------------')
print('Testing "1st Download Speed"')
print('-------------------------------------------------------------')
c = curl(url)
data = [c.getinfo(c.NAMELOOKUP_TIME),
        c.getinfo(c.CONNECT_TIME),
        c.getinfo(c.APPCONNECT_TIME),
        c.getinfo(c.PRETRANSFER_TIME),
        c.getinfo(c.REDIRECT_TIME),
        c.getinfo(c.STARTTRANSFER_TIME),
        c.getinfo(c.TOTAL_TIME),
        c.getinfo(c.TOTAL_TIME) ]
data = fixdata(data)
printresults(data)

print('-------------------------------------------------------------')
print('Testing "Cached Downloads Speeds"')
print('-------------------------------------------------------------')

# url = "https://d3va53q3li7xt1.cloudfront.net/wp-content/uploads/2021/05/shoeb-1024x576.png"
n = 10

responses = []
for i in range(n):
    c = curl(url)
    data = [c.getinfo(c.NAMELOOKUP_TIME),
        c.getinfo(c.CONNECT_TIME),
        c.getinfo(c.APPCONNECT_TIME),
        c.getinfo(c.PRETRANSFER_TIME),
        c.getinfo(c.REDIRECT_TIME),
        c.getinfo(c.STARTTRANSFER_TIME),
        c.getinfo(c.TOTAL_TIME),
        c.getinfo(c.TOTAL_TIME) ]
    data = fixdata(data)

    responses.append(data)
    c.close()


total_sum = [0,0,0,0,0,0,0,0]
for response in responses:
    for i in range(len(response)):
       total_sum[i] = total_sum[i] + response[i] 
        
results = [0,0,0,0,0,0,0,0]
for i in range(len(total_sum)):
    results[i] = total_sum[i]/n

print('{0} requests done. Average:'.format(n))
printresults(results)
